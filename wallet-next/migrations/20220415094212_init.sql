-- Add migration script here

CREATE TABLE penumbra (id INTEGER PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE chain_params(
    chain_id TEXT PRIMARY KEY, 
    epoch_duration BIGINT NOT NULL, 
    unbonding_epochs BIGINT NOT NULL,
    active_validator_limit BIGINT NOT NULL,
    slashing_penalty BIGINT NOT NULL,
    base_reward_rate BIGINT NOT NULL,
    ibc_enabled BOOLEAN,
    inbound_ics20_transfers_enabled BOOLEAN,
    outbound_ics20_transfers_enabled BOOLEAN
);