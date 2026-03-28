#![no_std]

#[cfg(test)] //
mod test;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, 
    Address, Env, Symbol, BytesN
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VestingError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidParam = 4,
    NotFound = 5,
    InsufficientBalance = 6,
    MathOverflow = 7,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VestingInfo {
    pub total_amount: i128,
    pub claimed: i128,
    pub start_time: u64,
    pub cliff_time: u64,
    pub duration: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Vesting(Address, u32),
    VestingCount(Address),
}

// TTL Constants (approx 30 days)
const DAY_IN_LEDGERS: u32 = 17280;
const PERSISTENT_EXTEND: u32 = 30 * DAY_IN_LEDGERS;
const PERSISTENT_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

// Event Symbols
const VESTING_CREATED: Symbol = symbol_short!("created");
const TOKENS_CLAIMED: Symbol = symbol_short!("claimed");

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Initialize the contract with an admin and the token to be vested.
    pub fn initialize(env: Env, admin: Address, token_address: Address) -> Result<(), VestingError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(VestingError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token_address);
        
        env.storage().instance().extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);
        Ok(())
    }

    /// Creates a new vesting schedule and transfers the total_amount from admin to the contract.
    pub fn create_vesting(
        env: Env,
        beneficiary: Address,
        total_amount: i128,
        start_time: u64,
        cliff_time: u64,
        duration: u64,
    ) -> Result<(), VestingError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(VestingError::NotInitialized)?;
        admin.require_auth();

        if total_amount <= 0 || duration == 0 { return Err(VestingError::InvalidParam); }
        if cliff_time < start_time { return Err(VestingError::InvalidParam); }
        
        let end_time = start_time.checked_add(duration).ok_or(VestingError::InvalidParam)?;
        if cliff_time > end_time { return Err(VestingError::InvalidParam); }

        // Transfer tokens from Admin to Contract
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&admin, &env.current_contract_address(), &total_amount);

        let count_key = DataKey::VestingCount(beneficiary.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        
        let info = VestingInfo {
            total_amount,
            claimed: 0,
            start_time,
            cliff_time,
            duration,
        };

        let key = DataKey::Vesting(beneficiary.clone(), count);
        env.storage().persistent().set(&key, &info);
        env.storage().persistent().set(&count_key, &(count + 1));
        
        // Extend TTLs
        env.storage().persistent().extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);
        env.storage().persistent().extend_ttl(&count_key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);
        env.storage().instance().extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);

        env.events().publish((VESTING_CREATED, beneficiary), (total_amount, start_time, duration));
        Ok(())
    }

    /// Claim unlocked tokens for a specific vesting schedule.
    pub fn claim(env: Env, beneficiary: Address, vesting_id: u32) -> Result<i128, VestingError> {
        beneficiary.require_auth();

        let key = DataKey::Vesting(beneficiary.clone(), vesting_id);
        let mut info: VestingInfo = env.storage().persistent().get(&key).ok_or(VestingError::NotFound)?;
        
        env.storage().persistent().extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);

        let claimable = Self::calculate_claimable(&env, &info)?;
        if claimable <= 0 { return Ok(0); }

        info.claimed = info.claimed.checked_add(claimable).ok_or(VestingError::MathOverflow)?;
        env.storage().persistent().set(&key, &info);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        
        token_client.transfer(&env.current_contract_address(), &beneficiary, &claimable);

        env.events().publish((TOKENS_CLAIMED, beneficiary), claimable);
        Ok(claimable)
    }

    /// Admin-only: Withdraw tokens from the contract in case of emergency.
    pub fn emergency_withdraw(env: Env, amount: i128) -> Result<(), VestingError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(VestingError::NotInitialized)?;
        admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        
        token_client.transfer(&env.current_contract_address(), &admin, &amount);
        Ok(())
    }

    // --- Helpers / View Functions ---

    /// Returns the amount of tokens currently available to claim.
    pub fn get_claimable_amount(env: Env, beneficiary: Address, vesting_id: u32) -> Result<i128, VestingError> {
        let key = DataKey::Vesting(beneficiary, vesting_id);
        let info: VestingInfo = env.storage().persistent().get(&key).ok_or(VestingError::NotFound)?;
        Self::calculate_claimable(&env, &info)
    }

    pub fn get_vesting(env: Env, beneficiary: Address, index: u32) -> Option<VestingInfo> {
        let key = DataKey::Vesting(beneficiary, index);
        let val = env.storage().persistent().get(&key);
        if val.is_some() {
            env.storage().persistent().extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND);
        }
        val
    }

    fn calculate_claimable(env: &Env, info: &VestingInfo) -> Result<i128, VestingError> {
        let now = env.ledger().timestamp();
        if now < info.cliff_time {
            return Ok(0);
        }

        let end_time = info.start_time.checked_add(info.duration).ok_or(VestingError::MathOverflow)?;
        
        let total_vested = if now >= end_time {
            info.total_amount
        } else {
            let elapsed = (now - info.start_time) as i128;
            info.total_amount
                .checked_mul(elapsed).ok_or(VestingError::MathOverflow)?
                .checked_div(info.duration as i128).ok_or(VestingError::MathOverflow)?
        };

        let claimable = total_vested.checked_sub(info.claimed).ok_or(VestingError::MathOverflow)?;
        Ok(claimable)
    }
}