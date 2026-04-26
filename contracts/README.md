# VacciChain Contract

## Token ID generation

Each vaccination NFT is assigned a deterministic `token_id` derived from its content and the minting ledger sequence. This prevents collisions, makes IDs auditable, and eliminates reliance on a mutable counter.

### Scheme

```
token_id = first_8_bytes_as_u64_be(
    SHA-256(
        patient_xdr
        || vaccine_name_xdr
        || date_administered_xdr
        || issuer_xdr
        || ledger_sequence_be4
    )
)
```

- `patient_xdr` — XDR-encoded patient `Address`
- `vaccine_name_xdr` — XDR-encoded Soroban `String` (vaccine name)
- `date_administered_xdr` — XDR-encoded Soroban `String` (date)
- `issuer_xdr` — XDR-encoded issuer `Address`
- `ledger_sequence_be4` — current ledger sequence number as 4 big-endian bytes

The resulting `token_id` is a `u64` (16 hex characters, zero-padded). The same inputs at the same ledger sequence always produce the same ID. Different inputs — including different patients, vaccines, dates, issuers, or ledger sequences — produce statistically independent IDs with negligible collision probability (birthday bound over 2^64).

Duplicate detection is enforced at the contract level: minting with an already-existing `token_id` or the same `(patient, vaccine_name, date_administered)` tuple returns `DuplicateRecord`.

## String input validation

The contract validates string inputs at the contract boundary before any storage operations.

### Limits

- `vaccine_name`: maximum 100 characters
- `date_administered`: maximum 100 characters
- `name` (issuer metadata): maximum 100 characters
- `license` (issuer metadata): maximum 100 characters
- `country` (issuer metadata): maximum 100 characters

### Errors

When a string input exceeds the configured limit, the contract returns an invalid input error variant:

- `InvalidInputVaccineName`
- `InvalidInputDateAdministered`
- `InvalidInputIssuerName`
- `InvalidInputLicense`
- `InvalidInputCountry`

This prevents excessively long strings from being stored on ledger state and keeps ledger fees bounded.
