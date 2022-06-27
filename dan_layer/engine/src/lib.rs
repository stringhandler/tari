pub struct InstructionProcessor {}

impl InstructionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct InstructionBuilder {
    method: Option<String>,
}

impl InstructionBuilder {
    pub fn new() -> Self {
        Self { method: None }
    }

    pub fn method<T: Into<String>>(mut self, method: T) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn build(self) -> Instruction {
        Instruction {
            method: self.method.expect("method not set"),
        }
    }
}

pub struct Instruction {
    method: String,
}

pub trait IntoContractState {
    fn register_state();
}
