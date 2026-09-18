#![no_std]

mod error;
mod storage;
mod types;

use soroban_sdk::contract;

#[contract]
pub struct Order;
