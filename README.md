# baggage_cover

## Project Title
baggage_cover

## Project Description
Travelers lose luggage on commercial flights more often than airlines admit, and the standard reimbursement process is slow, opaque, and almost impossible to dispute from a phone in a foreign airport. `baggage_cover` is a Soroban smart contract that lets a passenger buy a coverage policy for a specific bag on a specific flight, lets the authorized airline (or a trusted oracle) confirm the loss, and lets the passenger claim a fixed payout once the loss is on-chain confirmed. The contract stores the full policy lifecycle on the Stellar Testnet so that any dispute can be resolved from a public, tamper-proof record instead of an airline call center.

## Project Vision
The long-term goal is to make flight-based insurance as cheap and as fast to settle as sending a Stellar payment. By anchoring the policy, the loss report, and the payout claim in a single Soroban contract, `baggage_cover` aims to become a reference building block for the travel-insurance vertical on Stellar: an open primitive that any airline, OTA, or travel dApp can reuse without trusting a centralized claims processor.

## Key Features
- **Policy purchase (`buy_coverage`)** — A passenger authenticates and locks a `(flight_id, baggage_tag, coverage_amount)` triple in contract storage. Duplicate policies for the same flight and bag are rejected.
- **Loss reporting (`report_lost`)** — The airline that was authorized by the admin for a given flight is the only address that can confirm a loss, and it must attach a free-form `evidence_hash` (for example a pointer to an off-chain incident report or sensor log) at the time of the report.
- **Payout claim (`claim_payout`)** — Once the loss is confirmed, the original passenger is the only address that can claim the payout, and the contract returns the recorded `coverage_amount` while flipping the policy to `STATUS_CLAIMED` so it cannot be claimed twice.
- **Cancellation (`cancel`)** — A passenger can cancel an active policy before the loss is reported and attach a reason for audit.
- **Read-only views (`get_status`, `is_payable`)** — Any client can query the lifecycle of a policy or ask "is this bag currently payable?" without sending a transaction.
- **Admin-gated airline authorization (`init`, `authorize_airline`)** — A single admin deploys the contract and whitelists which airline address is trusted to report losses for each flight.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** travel dApp — see `contracts/baggage_cover/src/lib.rs` for the full baggage_cover business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `<CCWLBN2F4NCRDTET2AZUOCU7574HCEKGE7ESL2J6MPU75RVYFCECXUMT>`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/dec8b276e2585097734cfe972cc7891a9ff1cbabd9b4ebefca2eb529e622b10a`
- **Screenshot of deployed contract on Stellar Expert:**
  `_(https://prnt.sc/e1FI3D6xAgNi)_`


## Future Scope
- **Real asset payout** — Replace the recorded `coverage_amount` return value with an actual USDC transfer from a pre-funded treasury contract, gated by the same `STATUS_LOST -> STATUS_CLAIMED` transition.
- **Trustless oracle** — Replace the single authorized airline with a Reflector-oracle-style multi-signer report so that the loss is finalized only after N of M oracles agree.
- **Policy marketplace** — Let third-party insurers (not only the airline) underwrite a flight by depositing premium into the contract and let the contract distribute the premium when the policy is claimed or cancelled.
- **Time-bound coverage windows** — Use the ledger timestamp plus a configurable `coverage_until` field to auto-cancel policies whose flights have safely landed with no loss report.
- **Frontend dApp** — A Freighter-wallet-enabled web UI that walks a traveler through `buy_coverage`, surfaces the status of every active policy, and links directly to the Stellar Expert transaction for each step.
- **Audit dashboard** — An indexer that consumes the contract events and shows airlines a live view of outstanding, claimed, and cancelled policies per flight.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `baggage_cover` (travel)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
