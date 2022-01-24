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

use crate::{storage::state::StateDbUnitOfWork, DigitalAssetError};
use tari_crypto::common::Blake256;
use tari_common_types::types::PublicKey;
use digest::Digest;

const LOG_TARGET: &str = "tari::dan_layer::core::templates::tip10001_template";

pub fn invoke_method<TUnitOfWork: StateDbUnitOfWork>(
    method: String,
    args: &[u8],
    state_db: &mut TUnitOfWork,
) -> Result<(), DigitalAssetError> {
    match method.to_lowercase().replace("_", "").as_str() {
        "createlimit" => {
            let request =
                tip10001::CreateLimitRequest::decode(&*args).map_err(|e| DigitalAssetError::ProtoBufDecodeError {
                    source: e,
                    message_type: "tip10001::CreateLimitRequest".to_string(),
                })?;

            create_limit(request, state_db)?;
            Ok(())
        },
        _ => todo!(),
    }
}

pub fn invoke_read_method<TUnitOfWork: StateDbUnitOfWork>(
    method: String,
    args: &[u8],
    state_db: &mut TUnitOfWork,
) -> Result<Option<Vec<u8>>, DigitalAssetError> {
    match method.to_lowercase().replace("_", "").as_str() {
        "listlimits" => {
            let request =
                tip10001::ListLimitsRequest::decode(&*args).map_err(|e| DigitalAssetError::ProtoBufDecodeError {
                    source: e,
                    message_type: "tip10001::ListLimitsRequest".to_string(),
                })?;
            let (limits, total) = list_limits(request, state_db)?;
            let response = tip10001::ListLimitsResponse {
                limits,
                total
            };
            Ok(Some(response.encode_to_vec()))
        },
        _ => todo!(),
    }
}

fn list_limits<TUnitOfWork: StateDbUnitOfWork>(request: tip10001::ListLimitsRequest, state_db: &mut TUnitOfWork) ->Result<(Vec<Limit>, u32), DigitalAssetError> {
    let mut results = vec![];
    let mut total = 0;
    if request.asset_public_key.is_empty() {
       total = state_db.count("limits")?;
        for (hash, limit) in state_db.list_values("limits", request.page * request.page_size, request.page_size) {
           results.push(tip10001::Limit {
               maker_asset_public_key: limit.maker_asset_public_key,
               maker_amount: limit.maker_amount,
               maker_nft_id: limit.maker_nft_id,
               taker_amount_in_native: limit.taker_amount_in_native,
               ownership_proof: limit.ownership_proof,
               hash
           });
        }
    }
    else {
        todo!();
    }

   Ok((results, total))
}

fn create_limit<TUnitOfWork: StateDbUnitOfWork>(limit_request: tip10001::CreateLimitRequest, state_db: &mut TUnitOfWork) -> Result<(), DigitalAssetError> {
    // TODO: check proof of ownership
    let limit: Limit = limit_request.limit.unwrap().into();
    let hash = limit.hash();
    state_db.set_value("limits".to_string(), hash.clone(), bincode::serialize(limit))?;
    state_db.set_value(format!("limits::{}", limit.maker_asset_public_key), hash.clone(), vec![])?;
    state_db.set_value("owners".to_string(), hash, limit_request.owner)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Limit {
    maker_asset_public_key: PublicKey,
    maker_amount: u64,
    maker_nft_ids: Vec<Vec<u8>>,
    taker_amount_in_native: u64,
    ownership_proof: Vec<u8>
}

impl Limit {
    fn hash(&self) -> Vec<u8> {
        Blake256::new().chain(&self.maker_asset_public_key).chain(self.maker_amount).chain(&self.maker_nft_ids).chain(&self.taker_amount_in_native).chain(&self.ownership_proof).finalize().to_vec()
    }
}



impl From<tip10001::CreateLimitRequest> for Limit {
    fn from(source: tip10001::CreateLimitRequest) -> Self {
        Self {
                maker_asset_public_key: source.maker_asset_public_key,
            maker_amount: source.maker_amount,
            maker_nft_ids: source.maker_nft_ids,
            taker_amount_in_native: source.taker_amount_in_native,
            ownership_proof: source.ownership_proof
            }

        }
    }
}