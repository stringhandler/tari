// Copyright 2020. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::{
    dummy_data::get_dummy_contacts,
    utils::widget_states::{StatefulList, TabsState},
};
use log::*;
use qrcode::{render::unicode, QrCode};
use std::sync::Arc;
use tari_common::Network;
use tari_comms::NodeIdentity;
use tari_crypto::tari_utilities::hex::Hex;
use tari_wallet::{
    contacts_service::storage::{database::Contact, sqlite_db::ContactsServiceSqliteDatabase},
    output_manager_service::storage::sqlite_db::OutputManagerSqliteDatabase,
    storage::sqlite_db::WalletSqliteDatabase,
    transaction_service::storage::{database::CompletedTransaction, sqlite_db::TransactionServiceSqliteDatabase},
    util::emoji::EmojiId,
    Wallet,
};

pub struct App<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub wallet: Wallet<
        WalletSqliteDatabase,
        TransactionServiceSqliteDatabase,
        OutputManagerSqliteDatabase,
        ContactsServiceSqliteDatabase,
    >,
    // Cached state this will need to be cleaned up into a threadsafe container
    pub app_state: AppState,
    // Ui working state
    pub tabs: TabsState<'a>,
    pub selected_tx_list: SelectedTransactionList,
    pub to_field: String,
    pub amount_field: String,
    pub send_input_mode: SendInputMode,
    pub show_contacts: bool,
}

impl<'a> App<'a> {
    pub fn new(
        title: &'a str,
        wallet: Wallet<
            WalletSqliteDatabase,
            TransactionServiceSqliteDatabase,
            OutputManagerSqliteDatabase,
            ContactsServiceSqliteDatabase,
        >,
        network: Network,
    ) -> App<'a>
    {
        let app_state = AppState::new(wallet.comms.node_identity(), network);

        Self {
            title,
            wallet,
            should_quit: false,
            app_state,
            tabs: TabsState::new(vec!["Transactions", "Send/Receive", "Network"]),
            selected_tx_list: SelectedTransactionList::None,
            to_field: "".to_string(),
            amount_field: "".to_string(),
            send_input_mode: SendInputMode::None,
            show_contacts: false,
        }
    }

    pub fn on_control_key(&mut self, c: char) {
        match c {
            'c' => {
                self.should_quit = true;
            },
            _ => {},
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            },
            '\t' => {
                self.tabs.next();
                return;
            },
            _ => {},
        }
        if self.tabs.index == 0 {
            match c {
                'p' => {
                    self.selected_tx_list = SelectedTransactionList::PendingTxs;
                    self.app_state.pending_txs.select_first();
                    self.app_state.completed_txs.unselect();
                },
                'c' => {
                    self.selected_tx_list = SelectedTransactionList::CompletedTxs;
                    self.app_state.pending_txs.unselect();
                    self.app_state.completed_txs.select_first();
                },
                '\n' => match self.selected_tx_list {
                    SelectedTransactionList::None => {},
                    SelectedTransactionList::PendingTxs => {
                        self.app_state.detailed_transaction = self.app_state.pending_txs.selected_item();
                    },
                    SelectedTransactionList::CompletedTxs => {
                        self.app_state.detailed_transaction = self.app_state.completed_txs.selected_item();
                    },
                },
                _ => {},
            }
        }
        if self.tabs.index == 1 {
            match self.send_input_mode {
                SendInputMode::None => match c {
                    'c' => self.show_contacts = !self.show_contacts,
                    't' => self.send_input_mode = SendInputMode::To,
                    'a' => self.send_input_mode = SendInputMode::Amount,
                    '\n' => {
                        if self.show_contacts {
                            if let Some(c) = self.app_state.contacts.selected_item().as_ref() {
                                self.to_field = c.public_key.clone();
                                self.show_contacts = false;
                            }
                        }
                    },
                    _ => {},
                },
                SendInputMode::To => match c {
                    '\n' | '\t' => {
                        self.send_input_mode = SendInputMode::None;
                        self.send_input_mode = SendInputMode::Amount;
                    },
                    c => {
                        self.to_field.push(c);
                    },
                },
                SendInputMode::Amount => match c {
                    '\n' | '\t' => self.send_input_mode = SendInputMode::None,
                    c => {
                        if c.is_numeric() {
                            self.amount_field.push(c);
                        }
                    },
                },
            }
        }
    }

    pub fn on_up(&mut self) {
        if self.tabs.index == 0 {
            match self.selected_tx_list {
                SelectedTransactionList::None => {},
                SelectedTransactionList::PendingTxs => {
                    self.app_state.pending_txs.previous();
                    self.app_state.detailed_transaction = self.app_state.pending_txs.selected_item();
                },
                SelectedTransactionList::CompletedTxs => {
                    self.app_state.completed_txs.previous();
                    self.app_state.detailed_transaction = self.app_state.completed_txs.selected_item();
                },
            }
        }
        if self.tabs.index == 1 {
            self.app_state.contacts.previous();
        }
    }

    pub fn on_down(&mut self) {
        if self.tabs.index == 0 {
            match self.selected_tx_list {
                SelectedTransactionList::None => {},
                SelectedTransactionList::PendingTxs => {
                    self.app_state.pending_txs.next();
                    self.app_state.detailed_transaction = self.app_state.pending_txs.selected_item();
                },
                SelectedTransactionList::CompletedTxs => {
                    self.app_state.completed_txs.next();
                    self.app_state.detailed_transaction = self.app_state.completed_txs.selected_item();
                },
            }
        }
        if self.tabs.index == 1 {
            self.app_state.contacts.next();
        }
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_esc(&mut self) {
        if self.tabs.index == 1 {
            self.send_input_mode = SendInputMode::None;
            self.show_contacts = false;
        }
    }

    pub fn on_backspace(&mut self) {
        if self.tabs.index == 1 {
            match self.send_input_mode {
                SendInputMode::To => {
                    let _ = self.to_field.pop();
                },
                SendInputMode::Amount => {
                    let _ = self.amount_field.pop();
                },
                SendInputMode::None => {},
            }
        }
    }

    pub fn on_tick(&mut self) {}

    pub fn refresh_state(&mut self) {
        let mut pending_transactions: Vec<CompletedTransaction> = Vec::new();
        if let Ok(pending_inbound) = self
            .wallet
            .runtime
            .block_on(self.wallet.transaction_service.get_pending_inbound_transactions())
        {
            pending_transactions.extend(
                pending_inbound
                    .values()
                    .map(|t| CompletedTransaction::from(t.clone()))
                    .collect::<Vec<CompletedTransaction>>(),
            );
        }
        if let Ok(pending_outbound) = self
            .wallet
            .runtime
            .block_on(self.wallet.transaction_service.get_pending_inbound_transactions())
        {
            pending_transactions.extend(
                pending_outbound
                    .values()
                    .map(|t| CompletedTransaction::from(t.clone()))
                    .collect::<Vec<CompletedTransaction>>(),
            );
        }

        pending_transactions.sort_by(|a: &CompletedTransaction, b: &CompletedTransaction| {
            b.timestamp.partial_cmp(&a.timestamp).unwrap()
        });

        let completed_transactions = if let Ok(txs) = self
            .wallet
            .runtime
            .block_on(self.wallet.transaction_service.get_completed_transactions())
        {
            txs.values().map(|t| t.clone()).collect()
        } else {
            Vec::new()
        };

        self.app_state.pending_txs.items = pending_transactions;
        self.app_state.completed_txs.items = completed_transactions;
    }
}

pub struct AppState {
    pub pending_txs: StatefulList<CompletedTransaction>,
    pub completed_txs: StatefulList<CompletedTransaction>,
    pub detailed_transaction: Option<CompletedTransaction>,
    pub my_identity: MyIdentity,
    pub contacts: StatefulList<UiContact>,
}

impl AppState {
    pub fn new(node_identity: Arc<NodeIdentity>, network: Network) -> Self {
        let eid = EmojiId::from_pubkey(node_identity.public_key()).to_string();
        let qr_link = format!("tari://{}/pubkey/{}", network, &node_identity.public_key().to_hex());
        let code = QrCode::new(qr_link).unwrap();
        let image = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Dark)
            .light_color(unicode::Dense1x2::Light)
            .build()
            .trim()
            .to_string();

        let identity = MyIdentity {
            public_key: node_identity.public_key().to_string(),
            public_address: node_identity.public_address().to_string(),
            emoji_id: eid,
            qr_code: image,
        };
        AppState {
            pending_txs: StatefulList::new(),
            completed_txs: StatefulList::new(),
            detailed_transaction: None,
            my_identity: identity,
            contacts: StatefulList::with_items(
                get_dummy_contacts()
                    .iter()
                    .map(|c| UiContact::from(c.clone()))
                    .collect(),
            ),
        }
    }
}

#[derive(PartialEq)]
pub enum SelectedTransactionList {
    None,
    PendingTxs,
    CompletedTxs,
}

pub enum SendInputMode {
    None,
    To,
    Amount,
}

pub struct MyIdentity {
    pub public_key: String,
    pub public_address: String,
    pub emoji_id: String,
    pub qr_code: String,
}

#[derive(Clone)]
pub struct UiContact {
    pub alias: String,
    pub public_key: String,
    pub emoji_id: String,
}

impl From<Contact> for UiContact {
    fn from(c: Contact) -> Self {
        Self {
            alias: c.alias,
            public_key: format!("{}", c.public_key),
            emoji_id: EmojiId::from_pubkey(&c.public_key).as_str().to_string(),
        }
    }
}
