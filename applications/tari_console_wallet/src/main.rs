use crate::{
    app::App,
    utils::{
        crossterm_events::CrosstermEvents,
        events::{Event, EventStream},
    },
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::*;
use std::{
    io::{stdout, Write},
    sync::Arc,
};
use structopt::StructOpt;
use tari_app_utilities::utilities::{
    create_peer_db_folder,
    create_wallet_folder,
    parse_peer_seeds,
    setup_runtime,
    setup_wallet_transport_type,
    ExitCodes,
};
use tari_common::{ConfigBootstrap, GlobalConfig, Network};
use tari_core::{consensus::Network as NetworkType, transactions::types::CryptoFactories};

use tari_app_utilities::identity_management::setup_node_identity;
use tari_common::configuration::bootstrap::ApplicationType;
use tari_comms::{peer_manager::PeerFeatures, NodeIdentity};
use tari_comms_dht::{DbConnectionUrl, DhtConfig};
use tari_p2p::initialization::CommsConfig;
use tari_wallet::{
    error::WalletError,
    storage::sqlite_utilities::initialize_sqlite_database_backends,
    transaction_service::config::TransactionServiceConfig,
    wallet::WalletConfig,
    Wallet,
};
use tui::{backend::CrosstermBackend, Terminal};

#[macro_use]
extern crate lazy_static;

pub const LOG_TARGET: &str = "wallet::app::main";
/// The minimum buffer size for a tari application pubsub_connector channel
const BASE_NODE_BUFFER_MIN_SIZE: usize = 30;

mod app;
mod dummy_data;
mod ui;
mod utils;

/// Application entry point
fn main() {
    match main_inner() {
        Ok(_) => std::process::exit(0),
        Err(exit_code) => std::process::exit(exit_code as i32),
    }
}

fn main_inner() -> Result<(), ExitCodes> {
    // Parse and validate command-line arguments
    let mut bootstrap = ConfigBootstrap::from_args();

    // Check and initialize configuration files
    bootstrap.init_dirs(ApplicationType::ConsoleWallet)?;

    // Load and apply configuration file
    let cfg = bootstrap.load_configuration()?;

    // Initialise the logger
    bootstrap.initialize_logging()?;

    // Populate the configuration struct
    let node_config = GlobalConfig::convert_from(cfg).map_err(|err| {
        error!(target: LOG_TARGET, "The configuration file has an error. {}", err);
        ExitCodes::ConfigError
    })?;

    debug!(target: LOG_TARGET, "Using configuration: {:?}", node_config);
    // Load or create the Node identity
    let wallet_identity = setup_node_identity(
        &node_config.wallet_identity_file,
        &node_config.public_address,
        bootstrap.create_id ||
            // If the base node identity exists, we want to be sure that the wallet identity exists
            node_config.identity_file.exists(),
        PeerFeatures::COMMUNICATION_CLIENT,
    )?;

    // Exit if create_id or init arguments were run
    if bootstrap.create_id {
        info!(
            target: LOG_TARGET,
            "Node ID created at '{}'. Done.",
            node_config.identity_file.to_string_lossy()
        );
        return Ok(());
    }

    if bootstrap.init {
        info!(target: LOG_TARGET, "Default configuration created. Done.");
        return Ok(());
    }

    let app = setup_app(&node_config, wallet_identity)?;

    crossterm_loop(app)
}

/// Setup the app environment and state for use by the UI
fn setup_app(config: &GlobalConfig, node_identity: Arc<NodeIdentity>) -> Result<App, ExitCodes> {
    let runtime = setup_runtime(&config).map_err(|err| {
        error!(target: LOG_TARGET, "{}", err);
        ExitCodes::UnknownError
    })?;

    create_wallet_folder(
        &config
            .wallet_db_file
            .parent()
            .expect("wallet_db_file cannot be set to a root directory"),
    )
    .map_err(|e| {
        error!(target: LOG_TARGET, "Error creating Wallet folder. {}", e);
        ExitCodes::WalletError
    })?;
    create_peer_db_folder(&config.wallet_peer_db_path).map_err(|e| {
        error!(target: LOG_TARGET, "Error creating peer db folder. {}", e);
        ExitCodes::WalletError
    })?;

    debug!(target: LOG_TARGET, "Running Wallet database migrations");
    let (wallet_backend, transaction_backend, output_manager_backend, contacts_backend) =
        initialize_sqlite_database_backends(config.wallet_db_file.clone(), None).map_err(|e| {
            error!(target: LOG_TARGET, "Error creating Wallet database backends. {}", e);
            ExitCodes::WalletError
        })?;
    debug!(target: LOG_TARGET, "Databases Initialized");

    // TODO remove after next TestNet
    transaction_backend.migrate(node_identity.public_key().clone());

    let comms_config = CommsConfig {
        node_identity,
        user_agent: format!("tari/wallet/{}", env!("CARGO_PKG_VERSION")),
        transport_type: setup_wallet_transport_type(&config),
        datastore_path: config.wallet_peer_db_path.clone(),
        peer_database_name: "peers".to_string(),
        max_concurrent_inbound_tasks: 100,
        outbound_buffer_size: 100,
        // TODO - make this configurable
        dht: DhtConfig {
            database_url: DbConnectionUrl::File(config.data_dir.join("dht-wallet.db")),
            auto_join: true,
            ..Default::default()
        },
        // TODO: This should be false unless testing locally - make this configurable
        allow_test_addresses: true,
        listener_liveness_allowlist_cidrs: Vec::new(),
        listener_liveness_max_sessions: 0,
    };

    let network = match &config.network {
        Network::MainNet => NetworkType::MainNet,
        Network::Rincewind => NetworkType::Rincewind,
    };

    let factories = CryptoFactories::default();
    let mut wallet_config = WalletConfig::new(
        comms_config.clone(),
        factories,
        Some(TransactionServiceConfig {
            direct_send_timeout: comms_config.dht.discovery_request_timeout,
            ..Default::default()
        }),
        network,
    );
    wallet_config.buffer_size = std::cmp::max(BASE_NODE_BUFFER_MIN_SIZE, config.buffer_size_base_node);

    let mut wallet = Wallet::new(
        wallet_config,
        runtime,
        wallet_backend,
        transaction_backend.clone(),
        output_manager_backend,
        contacts_backend,
    )
    .map_err(|e| {
        if let WalletError::CommsInitializationError(ce) = e {
            error!(
                target: LOG_TARGET,
                "Error initializing Comms: {}",
                ce.to_friendly_string()
            );
        } else {
            error!(target: LOG_TARGET, "Error creating Wallet Container: {:?}", e);
        }

        ExitCodes::WalletError
    })?;

    // TODO update this to come from an explicit config field. This will be replaced by gRPC interface.
    if !config.peer_seeds.is_empty() {
        let seed_peers = parse_peer_seeds(&config.peer_seeds);
        wallet
            .set_base_node_peer(
                seed_peers[0].public_key.clone(),
                seed_peers[0]
                    .addresses
                    .first()
                    .expect("The seed peers should have an address")
                    .to_string(),
            )
            .map_err(|e| {
                error!(target: LOG_TARGET, "Error setting wallet base node peer. {}", e);
                ExitCodes::WalletError
            })?;
    }

    let mut app = App::new("Tari Console Wallet", wallet, config.network);
    app.refresh_state();

    Ok(app)
}

/// This is the main loop of the application UI using Crossterm based events
fn crossterm_loop(mut app: App) -> Result<(), ExitCodes> {
    let events = CrosstermEvents::new();
    enable_raw_mode().map_err(|e| {
        error!(target: LOG_TARGET, "Error enabling Raw Mode {}", e);
        ExitCodes::InterfaceError
    })?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| {
        error!(target: LOG_TARGET, "Error creating stdout context. {}", e);
        ExitCodes::InterfaceError
    })?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend).map_err(|e| {
        error!(target: LOG_TARGET, "Error creating Terminal context. {}", e);
        ExitCodes::InterfaceError
    })?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app)).map_err(|e| {
            error!(target: LOG_TARGET, "Error drawing interface. {}", e);
            ExitCodes::InterfaceError
        })?;

        match events.next().map_err(|e| {
            error!(target: LOG_TARGET, "Error reading input event: {}", e);
            ExitCodes::InterfaceError
        })? {
            Event::Input(event) => match (event.code, event.modifiers) {
                (KeyCode::Char(c), KeyModifiers::CONTROL) => app.on_control_key(c),
                (KeyCode::Char(c), _) => app.on_key(c),
                (KeyCode::Left, _) => app.on_left(),
                (KeyCode::Up, _) => app.on_up(),
                (KeyCode::Right, _) => app.on_right(),
                (KeyCode::Down, _) => app.on_down(),
                (KeyCode::Esc, _) => app.on_esc(),
                (KeyCode::Backspace, _) => app.on_backspace(),
                (KeyCode::Enter, _) => app.on_key('\n'),
                (KeyCode::Tab, _) => app.on_key('\t'),
                _ => {},
            },
            Event::Tick => {
                app.on_tick();
            },
        }
        if app.should_quit {
            break;
        }
    }

    disable_raw_mode().map_err(|e| {
        error!(target: LOG_TARGET, "Error disabling Raw Mode {}", e);
        ExitCodes::InterfaceError
    })?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| {
        error!(target: LOG_TARGET, "Error releasing stdout {}", e);
        ExitCodes::InterfaceError
    })?;
    terminal.show_cursor().map_err(|e| {
        error!(target: LOG_TARGET, "Error showing cursor: {}", e);
        ExitCodes::InterfaceError
    })?;

    println!("The wallet is shutting down.");
    info!(
        target: LOG_TARGET,
        "Termination signal received from user. Shutting wallet down."
    );

    app.wallet.shutdown();

    Ok(())
}
