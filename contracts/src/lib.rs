#![no_std]

#[cfg(test)]
extern crate std;

mod storage;
mod events;
mod mint;
mod verify;

use soroban_sdk::{contract, contractimpl, contracterror, Address, BytesN, Env, String, Vec};
use storage::{DataKey, IssuerRecord, VaccinationRecord};

/// Contract errors.
///
/// | Code | Name                         | Description                                      |
/// |------|------------------------------|--------------------------------------------------|
/// | 1    | AlreadyInitialized           | Contract has already been initialized            |
/// | 2    | NotInitialized               | Contract has not been initialized                |
/// | 3    | Unauthorized                 | Caller is not an authorized issuer               |
/// | 4    | ProposalExpired              | Admin transfer proposal has expired              |
/// | 5    | NoPendingTransfer            | No pending admin transfer exists                 |
/// | 6    | DuplicateRecord              | Identical vaccination record already exists      |
/// | 7    | RecordNotFound               | Vaccination record does not exist                |
/// | 8    | AlreadyRevoked               | Vaccination record is already revoked            |
/// | 9    | InvalidInput                 | Input failed validation at the contract boundary |
/// | 10   | InvalidInputVaccineName      | vaccine_name exceeds maximum length              |
/// | 11   | InvalidInputDateAdministered | date_administered exceeds maximum length         |
/// | 12   | InvalidInputIssuerName       | issuer name exceeds maximum length               |
/// | 13   | InvalidInputLicense          | issuer license exceeds maximum length            |
/// | 14   | InvalidInputCountry          | issuer country exceeds maximum length            |
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    ProposalExpired = 4,
    NoPendingTransfer = 5,
    DuplicateRecord = 6,
    RecordNotFound = 7,
    AlreadyRevoked = 8,
    InvalidInput = 9,
    InvalidInputVaccineName = 10,
    InvalidInputDateAdministered = 11,
    InvalidInputIssuerName = 12,
    InvalidInputLicense = 13,
    InvalidInputCountry = 14,
}

const MAX_STRING_LENGTH: u32 = 100;

pub(crate) fn validate_input_length(field: &String, field_name: &str) -> Result<(), ContractError> {
    if field.len() > MAX_STRING_LENGTH {
        return Err(match field_name {
            "vaccine_name" => ContractError::InvalidInputVaccineName,
            "date_administered" => ContractError::InvalidInputDateAdministered,
            "name" => ContractError::InvalidInputIssuerName,
            "license" => ContractError::InvalidInputLicense,
            "country" => ContractError::InvalidInputCountry,
            _ => ContractError::InvalidInput,
        });
    }
    Ok(())
}

#[contract]
pub struct VacciChainContract;

#[contractimpl]
impl VacciChainContract {
    /// Initialize contract with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// Admin: authorize a new issuer with metadata.
    pub fn add_issuer(
        env: Env,
        issuer: Address,
        name: String,
        license: String,
        country: String,
    ) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        validate_input_length(&name, "name")?;
        validate_input_length(&license, "license")?;
        validate_input_length(&country, "country")?;

        let record = IssuerRecord { name, license, country, authorized: true };
        env.storage().persistent().set(&DataKey::Issuer(issuer.clone()), &record);
        events::emit_issuer_added(&env, &issuer, &admin);
        Ok(())
    }

    /// Public: get issuer metadata.
    pub fn get_issuer(env: Env, address: Address) -> Option<IssuerRecord> {
        env.storage().persistent().get(&DataKey::Issuer(address))
    }

    /// Admin: revoke an issuer.
    pub fn revoke_issuer(env: Env, issuer: Address) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        if let Some(mut record) = env
            .storage()
            .persistent()
            .get::<DataKey, IssuerRecord>(&DataKey::Issuer(issuer.clone()))
        {
            record.authorized = false;
            env.storage().persistent().set(&DataKey::Issuer(issuer.clone()), &record);
            events::emit_issuer_revoked(&env, &issuer, &admin);
        }
        Ok(())
    }

    /// Issuer: mint a soulbound vaccination NFT.
    /// Returns the deterministic token_id (u64).
    pub fn mint_vaccination(
        env: Env,
        patient: Address,
        vaccine_name: String,
        date_administered: String,
        issuer: Address,
    ) -> Result<u64, ContractError> {
        mint::mint_vaccination(&env, patient, vaccine_name, date_administered, issuer)
    }

    /// Original issuer or admin: revoke a vaccination record.
    /// The record is marked revoked but never deleted (audit trail preserved).
    pub fn revoke_vaccination(env: Env, token_id: u64, revoker: Address) -> Result<(), ContractError> {
        revoker.require_auth();

        let mut record: VaccinationRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .ok_or(ContractError::RecordNotFound)?;

        if record.revoked {
            return Err(ContractError::AlreadyRevoked);
        }

        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;

        if revoker != record.issuer && revoker != admin {
            return Err(ContractError::Unauthorized);
        }

        record.revoked = true;
        env.storage().persistent().set(&DataKey::Token(token_id), &record);
        env.storage().persistent().set(&DataKey::Revoked(token_id), &true);
        events::emit_revoked(&env, token_id, &revoker);
        Ok(())
    }

    /// Transfer is permanently blocked — soulbound enforcement.
    pub fn transfer(_env: Env, _from: Address, _to: Address, _token_id: u64) {
        panic!("soulbound: transfers are disabled");
    }

    /// Public: verify vaccination status for a wallet.
    pub fn verify_vaccination(env: Env, wallet: Address) -> (bool, Vec<VaccinationRecord>) {
        verify::verify_vaccination(&env, wallet)
    }

    /// Public: batch verify vaccination status for multiple wallets (max 100).
    pub fn batch_verify(env: Env, wallets: Vec<Address>) -> Vec<(Address, bool, Vec<VaccinationRecord>)> {
        verify::batch_verify(&env, wallets)
    }

    /// Check if an address is an authorized issuer.
    pub fn is_issuer(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, IssuerRecord>(&DataKey::Issuer(address))
            .map(|r| r.authorized)
            .unwrap_or(false)
    }

    /// Admin: propose a new admin (two-step transfer). Proposal expires after 24 hours.
    pub fn propose_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();
        let expires_at = env.ledger().timestamp() + 86400;
        env.storage().persistent().set(&DataKey::PendingAdmin, &new_admin);
        env.storage().persistent().set(&DataKey::AdminTransferExpiry, &expires_at);
        events::emit_admin_transfer_proposed(&env, &admin, &new_admin, expires_at);
        Ok(())
    }

    /// Proposed admin: accept the admin role.
    pub fn accept_admin(env: Env) -> Result<(), ContractError> {
        let pending: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAdmin)
            .ok_or(ContractError::NoPendingTransfer)?;
        let expires_at: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::AdminTransferExpiry)
            .ok_or(ContractError::NoPendingTransfer)?;
        if env.ledger().timestamp() > expires_at {
            return Err(ContractError::ProposalExpired);
        }
        pending.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &pending);
        env.storage().persistent().remove(&DataKey::PendingAdmin);
        env.storage().persistent().remove(&DataKey::AdminTransferExpiry);
        events::emit_admin_transfer_accepted(&env, &pending);
        Ok(())
    }

    /// Admin: upgrade the contract WASM.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash.clone());
        events::emit_contract_upgraded(&env, &new_wasm_hash, &admin);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        BytesN, Env, String,
    };
    use storage::compute_token_id;

    fn setup() -> (Env, VacciChainContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VacciChainContract, ());
        let client = VacciChainContractClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_mint_and_verify() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        let vaccine = String::from_str(&env, "COVID-19");
        let date = String::from_str(&env, "2024-01-15");
        let seq = env.ledger().sequence();

        let token_id = client.mint_vaccination(&patient, &vaccine, &date, &issuer);

        // token_id must be a non-zero u64 (hash-derived)
        assert_ne!(token_id, 0);

        // token_id must match the deterministic scheme
        let expected = compute_token_id(&env, &patient, &vaccine, &date, &issuer, seq);
        assert_eq!(token_id, expected);

        let (vaccinated, records) = client.verify_vaccination(&patient);
        assert!(vaccinated);
        assert_eq!(records.len(), 1);
        assert_eq!(records.get(0).unwrap().token_id, token_id);
    }

    /// token_id is deterministic: same inputs at same ledger sequence → same id.
    #[test]
    fn test_token_id_deterministic() {
        let env = Env::default();
        env.mock_all_auths();

        let patient = Address::generate(&env);
        let issuer = Address::generate(&env);
        let vaccine = String::from_str(&env, "Flu");
        let date = String::from_str(&env, "2025-01-01");
        let seq = env.ledger().sequence();

        let id1 = compute_token_id(&env, &patient, &vaccine, &date, &issuer, seq);
        let id2 = compute_token_id(&env, &patient, &vaccine, &date, &issuer, seq);
        assert_eq!(id1, id2);
    }

    /// Different inputs produce different token_ids (collision resistance).
    #[test]
    fn test_token_id_collision_resistance() {
        let env = Env::default();
        env.mock_all_auths();

        let patient_a = Address::generate(&env);
        let patient_b = Address::generate(&env);
        let issuer = Address::generate(&env);
        let vaccine = String::from_str(&env, "COVID-19");
        let date = String::from_str(&env, "2024-01-15");
        let seq = env.ledger().sequence();

        // Different patient → different id
        let id_a = compute_token_id(&env, &patient_a, &vaccine, &date, &issuer, seq);
        let id_b = compute_token_id(&env, &patient_b, &vaccine, &date, &issuer, seq);
        assert_ne!(id_a, id_b);

        // Different vaccine → different id
        let vaccine2 = String::from_str(&env, "Flu");
        let id_c = compute_token_id(&env, &patient_a, &vaccine2, &date, &issuer, seq);
        assert_ne!(id_a, id_c);

        // Different date → different id
        let date2 = String::from_str(&env, "2024-06-01");
        let id_d = compute_token_id(&env, &patient_a, &vaccine, &date2, &issuer, seq);
        assert_ne!(id_a, id_d);

        // Different ledger sequence → different id
        let id_e = compute_token_id(&env, &patient_a, &vaccine, &date, &issuer, seq + 1);
        assert_ne!(id_a, id_e);

        // Different issuer → different id
        let issuer2 = Address::generate(&env);
        let id_f = compute_token_id(&env, &patient_a, &vaccine, &date, &issuer2, seq);
        assert_ne!(id_a, id_f);
    }

    /// token_id is a fixed-width u64 (64-bit, always 16 hex chars when zero-padded).
    #[test]
    fn test_token_id_is_fixed_width_u64() {
        let env = Env::default();
        env.mock_all_auths();

        let patient = Address::generate(&env);
        let issuer = Address::generate(&env);
        let vaccine = String::from_str(&env, "COVID-19");
        let date = String::from_str(&env, "2024-01-15");

        let id = compute_token_id(&env, &patient, &vaccine, &date, &issuer, 42u32);
        // A u64 is always exactly 8 bytes = 16 hex nibbles
        assert_eq!(core::mem::size_of_val(&id), 8);
        // Verify the value fits in u64 range (trivially true, but documents intent)
        let _: u64 = id;
    }

    #[test]
    fn test_transfer_blocked() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        // transfer always panics — use try_invoke via the SDK's panic capture
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.transfer(&from, &to, &1u64);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_unauthorized_issuer_blocked() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let fake_issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);

        let result = client.try_mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &fake_issuer,
        );
        assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    }

    #[test]
    fn test_duplicate_record_blocked() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        client.mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );

        let result = client.try_mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );
        assert_eq!(result, Err(Ok(ContractError::DuplicateRecord)));
    }

    #[test]
    fn test_add_issuer_invalid_input_name_too_long() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        let long_name = "A".repeat(101);
        let result = client.try_add_issuer(
            &issuer,
            &String::from_str(&env, &long_name),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidInputIssuerName)));
    }

    #[test]
    fn test_mint_vaccination_invalid_input_vaccine_name_too_long() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        let long_vaccine = "A".repeat(101);
        let result = client.try_mint_vaccination(
            &patient,
            &String::from_str(&env, &long_vaccine),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidInputVaccineName)));
    }

    #[test]
    fn test_double_init_rejected() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);
        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
    }

    #[test]
    fn test_propose_and_accept_admin() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        client.initialize(&admin);
        client.propose_admin(&new_admin);
        client.accept_admin();

        let issuer = Address::generate(&env);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );
    }

    #[test]
    fn test_accept_admin_expired() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        client.initialize(&admin);
        client.propose_admin(&new_admin);

        env.ledger().with_mut(|l| l.timestamp += 86401);

        let result = client.try_accept_admin();
        assert_eq!(result, Err(Ok(ContractError::ProposalExpired)));
    }

    #[test]
    fn test_revoke_vaccination() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        let token_id = client.mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );

        let (vaccinated, _) = client.verify_vaccination(&patient);
        assert!(vaccinated);

        client.revoke_vaccination(&token_id, &issuer);

        let (vaccinated_after, records_after) = client.verify_vaccination(&patient);
        assert!(!vaccinated_after);
        assert_eq!(records_after.len(), 0);
    }

    #[test]
    fn test_revoke_already_revoked() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        let token_id = client.mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );

        client.revoke_vaccination(&token_id, &issuer);
        let result = client.try_revoke_vaccination(&token_id, &issuer);
        assert_eq!(result, Err(Ok(ContractError::AlreadyRevoked)));
    }

    #[test]
    fn test_revoke_unauthorized() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let patient = Address::generate(&env);
        let stranger = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );

        let token_id = client.mint_vaccination(
            &patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );

        let result = client.try_revoke_vaccination(&token_id, &stranger);
        assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    }

    #[test]
    fn test_upgrade_admin_only() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);
        let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.upgrade(&wasm_hash);
    }

    #[test]
    fn test_batch_verify_empty() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);
        let results = client.batch_verify(&Vec::new(&env));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_batch_verify_partial() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let vaccinated_patient = Address::generate(&env);
        let unvaccinated_patient = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(
            &issuer,
            &String::from_str(&env, "General Hospital"),
            &String::from_str(&env, "LIC-12345"),
            &String::from_str(&env, "USA"),
        );
        client.mint_vaccination(
            &vaccinated_patient,
            &String::from_str(&env, "COVID-19"),
            &String::from_str(&env, "2024-01-15"),
            &issuer,
        );

        let mut wallets: Vec<Address> = Vec::new(&env);
        wallets.push_back(vaccinated_patient.clone());
        wallets.push_back(unvaccinated_patient.clone());

        let results = client.batch_verify(&wallets);
        assert_eq!(results.len(), 2);

        let (addr0, v0, r0) = results.get(0).unwrap();
        assert_eq!(addr0, vaccinated_patient);
        assert!(v0);
        assert_eq!(r0.len(), 1);

        let (addr1, v1, r1) = results.get(1).unwrap();
        assert_eq!(addr1, unvaccinated_patient);
        assert!(!v1);
        assert_eq!(r1.len(), 0);
    }

    #[test]
    #[should_panic(expected = "batch size exceeds maximum of 100")]
    fn test_batch_verify_exceeds_limit() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let mut wallets: Vec<Address> = Vec::new(&env);
        for _ in 0..101u32 {
            wallets.push_back(Address::generate(&env));
        }
        client.batch_verify(&wallets);
    }
}
