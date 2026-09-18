#![no_std]

mod storage;
mod types;

use soroban_sdk::contract;

#[contract]
pub struct Order;
