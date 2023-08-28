#![no_main]

use libfuzzer_sys::fuzz_target;
use tari_script::{self, ScriptError};

fuzz_target!(|data: &[u8]| {
    match tari_script::TariScript::from_bytes(data) {
        Ok(s) => {
            if data.len() == 0 {
                return;
            }
            // if s.size() == 0 {
            //     return;
            // }
            use tari_script::Opcode::*;
            match data[0] {
                // OP_CHECK_HEIGHT_VERIFY => {
                // },
                _ => {
                    // Test round trip
                    let v = s.to_bytes();
                    assert_eq!(v, &data[0..v.len()]);
                },
            }
        },
        Err(e) => {
            //  dbg!(&e);
            match e {
                ScriptError::InvalidOpcode => {
                    // ok I guess
                    //        dbg!(data[0]);
                },
                ScriptError::InvalidData => {
                    //       dbg!(data[0]);
                },
                _ => {
                    todo!();
                },
            }
        },
    }

    // fuzz some specific op_codes
    if data.len() > 32 {
        let mut v = vec![0x7a];
        v.extend_from_slice(&data[0..32]);
        let res = tari_script::TariScript::from_bytes(&v).unwrap();
    }
});
