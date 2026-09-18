#![no_std]

mod error;
mod events;
mod storage;
mod types;

use soroban_sdk::contract;

#[contract]
pub struct Registry;
