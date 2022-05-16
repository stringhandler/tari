# RFC0000-Confidential Assets

## A possible implementation for Confidential Assets on the Tari Digital Asset Network

![status: raw](theme/images/status-raw.svg)

**Maintainer(s)**: Mike Berry @mikethetike

# Licence

[ The 3-Clause BSD Licence](https://opensource.org/licenses/BSD-3-Clause).

Copyright 2022  The Tari Development Community

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
following conditions are met:

1. Redistributions of this document must retain the above copyright notice, this list of conditions and the following
   disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following
   disclaimer in the documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products
   derived from this software without specific prior written permission.

THIS DOCUMENT IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS", AND ANY EXPRESS OR IMPLIED WARRANTIES,
INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
WHETHER IN CONTRACT, STRICT LIABILITY OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## Language

The keywords "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", 
"NOT RECOMMENDED", "MAY" and "OPTIONAL" in this document are to be interpreted as described in 
[BCP 14](https://tools.ietf.org/html/bcp14) (covering RFC2119 and RFC8174) when, and only when, they appear in all capitals, as 
shown here.

## Disclaimer

This document and its content are intended for information purposes only and may be subject to change or update
without notice.

This document may include preliminary concepts that may or may not be in the process of being developed by the Tari
community. The release of this document is intended solely for review and discussion by the community regarding the
technological merits of the potential system outlined herein.

## Goals
1. To create a method of interacting on the DAN layer, hiding the sender, receiver and amounts from transactions
2. To enable assertions of ownership for parties using only the base layer
3. To enable "freezing" of assets from the DAN layer to the base layer

## Related Requests for Comment

## Description

In this scheme, we assume a sidechain running the second layer is using UTXO's.

#### Addresses

Each address on the sidechain is made up of a ViewKeyPair(v_k, VK) and a SpendKeyPair
(s_k, SK).

When receiving funds in the form of extended commitments, the sender creates a 
one time address from the receiver's address as follows:

1. Create nonce r_k and public nonce RK
2. Calculate r_k*VK + SK = AK
3. Address is (RK,AK) (referred to as owner)

To spend a UTXO, the owner provides a signature using a_k*RK + sk.

### State

The sidechain state is defined
```rust
struct Sidechain {
   utxos: Vec<Utxo>,
   last_checkpoint_utxo_length: u64,
   is_spent: RoaringBitmap,
   is_burnt: RoaringBitmap
}

struct Utxo {
   commitment: ExtendedCommitment,
   owner: PublicKey
}
```


When creating a checkpoint on the base layer, the following structure is created:

```rust 
struct Checkpoint {
    previous_utxo_mmr_root: MmrRoot,
    new_utxos_mmr_root: MmrRoot,
    mmr_root: MmrRoot,
    is_spent: RoaringBitmap,
    is_burnt: RoaringBitmap,
    new_burns: RoaringBitmap 
}
```

Note: when creating the MerkleMountainRanges: the hash is calculated as:

```
Hash(mmr_position | utxo | owner)
```
The position is useful for proving it's position in 
the `is_spent` and `is_burnt` bitmaps.

At every checkpoint, a new UTXO MMR is started. The MMR root field is calculated from 
the previous MMR_root and the current UTXO MMR. This allows users to update
ownership proofs they have generated from previous checkpoints.

### Proving ownership

A user can create a proof of ownership for a second layer asset




New base layer consensus rule:
1. When validating a new Checkpoint UTXO, the base layer must check that the previous 
mmr root is equal to the mmr root from the checkpoint being spent.
2. In the tx, there must be `new_burns.count()` new frozen utxos: 
   1. Each UTXO will contain an extended commitment
   2. Each UTXO must provide a merkle proof to `mmr_root`
   3. The relevant `is_burnt` position must be 1.
   4. A signature to prove ownership. 

The following operations are defined:



**Transfer(inputs, outputs)**: 
   - inputs is an array of tuples: (input_utxo, spending_proof)
   - outputs is an array of tuples: (output_utxo, address, rangeproofs)

**Burn(utxo, ownership_proof)**
  - 

