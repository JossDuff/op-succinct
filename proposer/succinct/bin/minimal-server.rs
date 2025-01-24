use alloy_primitives::B256;
use anyhow::Result;
use log::{error, info};
use op_succinct_client_utils::types::u32_to_u8;
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher, RunContext},
    get_proof_stdin,
    witnessgen::{WitnessGenExecutor, WITNESSGEN_TIMEOUT},
    ProgramType,
};
use op_succinct_proposer::{
    SpanProofRequest
};
use sp1_sdk::{
    Prover,
    HashableKey,
    utils, ProverClient 
};
use std::{str::FromStr, fs::File, io::Write}; 


pub const RANGE_ELF: &[u8] = include_bytes!("../../../elf/range-elf");
pub const AGG_ELF: &[u8] = include_bytes!("../../../elf/aggregation-elf");

#[tokio::main]
async fn main() -> Result<()> {

    // dummy payload
    let payload = SpanProofRequest {
        start: 243027,
        end: 243028
    };
    utils::setup_logger();

    dotenv::dotenv().ok();

    let prover = ProverClient::from_env();
    let (range_pk, range_vk) = prover.setup(RANGE_ELF);
    // let (_agg_pk, agg_vk) = prover.setup(AGG_ELF);
    let multi_block_vkey_u8 = u32_to_u8(range_vk.vk.hash_u32());
    // let _range_vkey_commitment = B256::from(multi_block_vkey_u8);
    // let _agg_vkey_hash = B256::from_str(&agg_vk.bytes32()).unwrap();

    let fetcher = match OPSuccinctDataFetcher::new_with_rollup_config(RunContext::Docker).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to create data fetcher: {}", e);
            todo!();
        }
    };

    let host_cli = match fetcher
        .get_host_cli_args(
            payload.start,
            payload.end,
            ProgramType::Multi,
            CacheMode::DeleteCache,
        )
        .await
    {
        Ok(cli) => cli,
        Err(e) => {
            error!("Failed to get host CLI args: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to get host CLI args: {}",
                e
            ));
        }
    };

    // Start the server and native client with a timeout.
    // Note: Ideally, the server should call out to a separate process that executes the native
    // host, and return an ID that the client can poll on to check if the proof was submitted.
    let mut witnessgen_executor = WitnessGenExecutor::new(WITNESSGEN_TIMEOUT, RunContext::Docker);
    if let Err(e) = witnessgen_executor.spawn_witnessgen(&host_cli).await {
        error!("Failed to spawn witness generation: {}", e);
        return Err(anyhow::anyhow!(
            "Failed to spawn witness generation: {}",
            e
        ));
    }
    // Log any errors from running the witness generation process.
    if let Err(e) = witnessgen_executor.flush().await {
        error!("Failed to generate witness: {}", e);
        return Err(anyhow::anyhow!(
            "Failed to generate witness: {}",
            e
        ));
    }

    let sp1_stdin = match get_proof_stdin(&host_cli) {
        Ok(stdin) => stdin,
        Err(e) => {
            error!("Failed to get proof stdin: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to get proof stdin: {}",
                e
            ));
        }
    };

    /*
    let serialized = bincode::serialize(&sp1_stdin).unwrap();
    let mut file = File::create("new-stdin.bin").unwrap();
    file.write_all(&serialized).unwrap();

    let sp1_stdin = {
        let stdin_bytes = std::fs::read("new-stdin.bin").unwrap();
        bincode::deserialize(&stdin_bytes).unwrap()
    };
    */

    info!("executing span proof");

    let _proof_res = prover.prove(&range_pk, &sp1_stdin).compressed().run().unwrap();
    
    info!("done with proof");

    Ok(())
}

