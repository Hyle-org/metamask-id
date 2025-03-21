#![no_main]
#![no_std]

extern crate alloc;

use sdk::guest::execute;
use sdk::guest::GuestEnv;
use sdk::guest::Risc0Env;

use contract_identity::IdentityContractState;

risc0_zkvm::guest::entry!(main);

fn main() {
    let env = Risc0Env {};
    let input = env.read();
    let (_, output) = execute::<IdentityContractState>(&input);
    env.commit(&output);
}
