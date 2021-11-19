//  Copyright 2021. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

pub mod models;
pub mod sqlite;
mod storage_error;

use crate::storage::models::{asset_row::AssetRow, wallet_row::WalletRow};
pub use storage_error::StorageError;
use tari_key_manager::cipher_seed::CipherSeed;
use uuid::Uuid;

pub trait CollectiblesStorage {
  type Addresses: AddressesTableGateway;
  type Assets: AssetsTableGateway;
  type AssetWallets: AssetWalletsTableGateway;
  type IssuedAssets: IssuedAssetsTableGateway;
  type Tip002Addresses: Tip002AddressesTableGateway;
  type Wallets: WalletsTableGateway;

  fn addresses(&self) -> Self::Addresses;
  fn assets(&self) -> Self::Assets;
  fn asset_wallets(&self) -> Self::AssetWallets;
  fn issued_assets(&self) -> Self::IssuedAssets;
  fn tip002_addresses(&self) -> Self::Tip002Addresses;
  fn wallets(&self) -> Self::Wallets;
}

pub trait AssetsTableGateway {
  fn list(&self) -> Result<Vec<AssetRow>, StorageError>;
  fn insert(&self, asset: AssetRow) -> Result<(), StorageError>;
  fn find(&self, asset_id: Uuid) -> Result<AssetRow, StorageError>;
}

pub trait WalletsTableGateway {
  type Passphrase;

  fn list(&self) -> Result<Vec<WalletRow>, StorageError>;
  fn insert(&self, wallet: WalletRow, pass: Self::Passphrase) -> Result<(), StorageError>;
  fn find(&self, id: Uuid) -> Result<WalletRow, StorageError>;
  fn get_cipher_seed(&self, id: Uuid, pass: Self::Passphrase) -> Result<CipherSeed, StorageError>;
}

pub trait AssetWalletsTableGateway {}

pub trait AddressesTableGateway {}

pub trait IssuedAssetsTableGateway {}

pub trait Tip002AddressesTableGateway {}