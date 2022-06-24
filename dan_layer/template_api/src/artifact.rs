//  Copyright 2022. The Tari Project
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

define_asset! {
    Artifact extends Owned {
        #[unique]
        message: TariString
    }

    ContractState {
        states: Vec<TariString>,
        current_state: usize
    }
}

define_action! {
    ContractStateOperations {
        #[init]
        fn init(states: Vec<TariString>, current_state: usize) -> Self {
            Self {
                states,
                current_state
            }
        }

        #[writable]
        fn change_state(&mut self, new_state: usize) -> Result<(), ActionError> {
            self.current_state = new_state;
        }

        #[readable]
        fn current_state(&self) -> &TariString {
           self.states[self.current_state]
        }

        #[readable]
        fn current_state_raw(&self) -> usize {
            self.current_state
        }
    }
}

define_action! {
    ArtifactOperations {
        #[writable]
        #[auth(Roles: [ARTIFACT_MINT])]
        fn mint(context: &OperationContext, expiry: u64, message: TariString) -> Result<(), ActionError> {
            if expiry < context.blocks.timestamp {
                return Err(ActionError::InvalidArgument("expiry time has elapsed"));
            }
            let artifact : Bucket<Artifact> = context.minter.mint(message);
            let mut sender_vault = context.sender.get_vault::<Artifact>();
            sender_vault.store(artifact);
            Ok(())
        }

        #[writable]
        #[auth(Roles: [ARTIFACT_BURN])]
        fn burn(&self, context: &OperationContext) -> Result<(), ActionError> {
            // Must have set up contract state somehow
           if context.contract_state.current_state
        }
    }
}
