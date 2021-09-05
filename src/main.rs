use {
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey,
        signer::{
            keypair::{read_keypair_file, write_keypair_file, Keypair},
            Signer,
        },
        transaction::Transaction,
        {borsh::try_from_slice_unchecked, program_pack::Pack},
    },
    spl_token::state::{Account, Mint},
    spl_token_metadata::{
        instruction::{create_master_edition, create_metadata_accounts},
        state::{Metadata, EDITION, PREFIX},
    },
    std::{io, io::Write, thread, time},
};

const CLIENT_URL: &'static str = "https://api.devnet.solana.com";
const WALLET_FILE_PATH: &'static str = "wallet.keypair";

fn get_wallet() -> Keypair {
    let wallet_keypair: Keypair = if let Ok(keypair) = read_keypair_file(WALLET_FILE_PATH) {
        keypair
    } else {
        let new_keypair = Keypair::new();
        write_keypair_file(&new_keypair, WALLET_FILE_PATH).unwrap();
        new_keypair
    };

    return wallet_keypair;
}

fn create_mint_account(wallet_keypair: &Keypair, client: &RpcClient) -> Pubkey {
    let mint_account: Keypair = Keypair::new();
    let mint_account_pubkey = mint_account.pubkey();
    let wallet_pubkey = wallet_keypair.pubkey();

    let minimum_balance_for_rent_exemption = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .unwrap();

    let create_account_instruction: Instruction = solana_sdk::system_instruction::create_account(
        &wallet_pubkey,
        &mint_account_pubkey,
        minimum_balance_for_rent_exemption,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    let initialize_mint_instruction: Instruction = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_account_pubkey,
        &wallet_pubkey,
        None,
        0,
    )
    .unwrap();

    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash().unwrap();

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &vec![create_account_instruction, initialize_mint_instruction],
        Some(&wallet_pubkey),
        &[&mint_account, &wallet_keypair],
        recent_blockhash,
    );

    let result = client.send_and_confirm_transaction_with_spinner(&transaction);

    if result.is_ok() {
        println!(
            "Successfully created a Mint Account with Pubkey: {:?}",
            mint_account_pubkey
        )
    };

    return mint_account_pubkey;
}

fn create_token_account(
    wallet_keypair: &Keypair,
    mint_account_pubkey: &Pubkey,
    client: &RpcClient,
) -> Pubkey {
    let wallet_pubkey = wallet_keypair.pubkey();
    let account_mint_to: Keypair = Keypair::new();
    let account_mint_to_pubkey: Pubkey = account_mint_to.pubkey();

    let create_account_instruction: Instruction = solana_sdk::system_instruction::create_account(
        &wallet_pubkey,
        &account_mint_to_pubkey,
        client
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .unwrap(),
        Account::LEN as u64,
        &spl_token::id(),
    );
    let initialize_account2_instruction: Instruction = spl_token::instruction::initialize_account2(
        &spl_token::id(),
        &account_mint_to_pubkey,
        &mint_account_pubkey,
        &wallet_pubkey,
    )
    .unwrap();

    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash().unwrap();

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &vec![create_account_instruction, initialize_account2_instruction],
        Some(&wallet_pubkey),
        &[&wallet_keypair, &account_mint_to],
        recent_blockhash,
    );

    let result = client.send_and_confirm_transaction_with_spinner(&transaction);
    if result.is_ok() {
        println!(
            "Successfully created a Token Account with Pubkey: {:?}",
            account_mint_to_pubkey
        )
    };

    return account_mint_to_pubkey;
}

fn mint_nft(
    wallet_keypair: &Keypair,
    mint_account_pubkey: &Pubkey,
    token_account_pubkey: &Pubkey,
    client: &RpcClient,
) {
    let wallet_pubkey = wallet_keypair.pubkey();

    let mint_to_instruction: Instruction = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_account_pubkey,
        &token_account_pubkey,
        &wallet_pubkey,
        &[&wallet_pubkey],
        1,
    )
    .unwrap();

    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash().unwrap();
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &vec![mint_to_instruction],
        Some(&wallet_pubkey),
        &[wallet_keypair],
        recent_blockhash,
    );

    let result = client.send_and_confirm_transaction_with_spinner(&transaction);
    if result.is_ok() {
        println!("Successfully Minted NFT to : {:?}", wallet_pubkey);

        upgrade_to_master_edition(
            &wallet_keypair,
            &create_metadata_account(&wallet_keypair, &mint_account_pubkey, &client),
            &mint_account_pubkey,
            &client,
        );
    };
}

fn create_metadata_account(
    wallet_keypair: &Keypair,
    mint_account_pubkey: &Pubkey,
    client: &RpcClient,
) -> Pubkey {
    let wallet_pubkey = wallet_keypair.pubkey();

    let program_key = spl_token_metadata::id();
    let metadata_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        mint_account_pubkey.as_ref(),
    ];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);

    // Test Metadata
    let name = String::from("Will Coin");
    let symbol = String::from("W");
    let uri = String::from("https://solana.com");

    let new_metadata_instruction = create_metadata_accounts(
        program_key,
        metadata_key,
        *mint_account_pubkey,
        wallet_pubkey,
        wallet_pubkey,
        wallet_pubkey,
        name,
        symbol,
        uri,
        None,
        0,
        false,
        false,
    );

    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash().unwrap();

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &vec![new_metadata_instruction],
        Some(&wallet_pubkey),
        &[wallet_keypair],
        recent_blockhash,
    );

    let result = client.send_and_confirm_transaction_with_spinner(&transaction);
    if result.is_ok() {
        println!(
            "Successfully created a new Metadata Account with Pubkey: {:?}",
            metadata_key
        )
    };

    return metadata_key;
}

fn upgrade_to_master_edition(
    wallet_keypair: &Keypair,
    metadata_key: &Pubkey,
    mint_account_pubkey: &Pubkey,
    client: &RpcClient,
) {
    let wallet_pubkey = wallet_keypair.pubkey();
    let program_key = spl_token_metadata::id();

    let metadata_account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&metadata_account.data).unwrap();

    let master_edition_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        &metadata.mint.as_ref(),
        EDITION.as_bytes(),
    ];
    let (master_edition_key, _) = Pubkey::find_program_address(master_edition_seeds, &program_key);

    let master_edition_instruction = create_master_edition(
        program_key,
        master_edition_key,
        *mint_account_pubkey,
        wallet_pubkey,
        wallet_pubkey,
        *metadata_key,
        wallet_pubkey,
        Some(1),
    );

    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash().unwrap();
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &vec![master_edition_instruction],
        Some(&wallet_pubkey),
        &[wallet_keypair],
        recent_blockhash,
    );

    let result = client.send_and_confirm_transaction_with_spinner(&transaction);

    if result.is_ok() {
        println!("Upgraded Metadata Account to Master Edition!");
    } else {
        println!("{:?}", result);
        return;
    }

    let master_metadata: Metadata = try_from_slice_unchecked(&metadata_account.data).unwrap();

    println!("\nSnapshot of Master Edition Metadata\n");
    println!("key: {:#?}", master_metadata.key);
    println!("update_authority: {:#?}", master_metadata.update_authority);
    println!("mint: {:#?}", master_metadata.mint);
    println!(
        "name: {:#?}",
        master_metadata.data.name.trim_end_matches(char::from(0))
    );
    println!(
        "symbol: {:#?}",
        master_metadata.data.symbol.trim_end_matches(char::from(0))
    );
    println!(
        "uri: {:#?}",
        master_metadata.data.uri.trim_end_matches(char::from(0))
    );
    println!(
        "seller_fee_basis_points: {:#?}",
        master_metadata.data.seller_fee_basis_points
    );
}

fn main() {
    // Get our Wallet KeyPair
    let wallet_keypair = get_wallet();
    let wallet_pubkey: Pubkey = wallet_keypair.pubkey();

    let program_key = spl_token_metadata::id();
    println!("{:?}", program_key);

    // Connect to the Solana Client and pull our wallet balance
    let client = RpcClient::new(CLIENT_URL.to_string());
    let wallet_balance = client.get_balance(&wallet_pubkey).unwrap();

    println!("Wallet Pubkey: {}", wallet_pubkey);
    println!("Wallet Balance: {}", wallet_balance);

    // Airdrop funds if our wallet is empty
    if wallet_balance == 0 {
        let result = client.request_airdrop(&wallet_keypair.pubkey(), 10_000_000_000);

        if result.is_ok() {
            print!("Airdropping funds to {:?}", wallet_pubkey);
            io::stdout().flush().unwrap();
            while client.get_balance(&wallet_pubkey).unwrap() == 0 {
                print!(".");
                io::stdout().flush().unwrap();
                let one_second = time::Duration::from_millis(1000);
                thread::sleep(one_second);
            }
            println!("");
        } else {
            println!("Failed to Airdrop funds. Try again later.");
            return;
        }
    }

    // Create the required prelim accounts
    let mint_account_pubkey = create_mint_account(&wallet_keypair, &client);
    let token_account_pubkey = create_token_account(&wallet_keypair, &mint_account_pubkey, &client);

    // Create the NFT, including the Metadata associated with it
    mint_nft(
        &wallet_keypair,
        &mint_account_pubkey,
        &token_account_pubkey,
        &client,
    );

    return;
}
