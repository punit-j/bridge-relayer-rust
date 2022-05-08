extern crate serde;

use std::fs;
use std::str::FromStr;
use web3::contract::{Contract, Options};
use web3::ethabi::Uint;

pub async fn doit() {
    let transport = web3::transports::Http::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5").unwrap();
    let client = web3::Web3::new(transport);

    println!("Calling accounts.");
    let mut accounts = client.eth().accounts().await.unwrap();
    println!("Accounts: {:?}", accounts);

    let my_addr = web3::types::Address::from_str("51599eC779c5fd6b59c5aCc6a31D8e174D8A793E").unwrap();
    let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();

    let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
    let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();
    //accounts.push(a);
    println!("aaaa {}", my_addr);

    let b = client.eth().balance(my_addr, Option::None).await;
    println!("bal {:?}", b);

    let abi = fs::read("/home/misha/trash/abi.json").unwrap();
    let contract = web3::contract::Contract::from_json(client.eth(), contract_addr, &*abi);

    println!("contr {:?}", contract);
    let contract = contract.unwrap();

    let res: web3::types::Address = contract.query("owner", (), None, Default::default(), None).await.unwrap();
    println!("owner {:?}", res);

    let res: bool = contract.query("isTokenInWhitelist", (token_addr, ),
                                   None, Default::default(), None).await.unwrap();
    println!("isTokenInWhitelist {:?}", res);

    let res = contract.signed_call("transferTokens", (token_addr,
                                                      web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),  // to
                                                      Uint::from(112),
                                                      Uint::from(10)),    // amount
                                                      Default::default(),
                                                      &priv_key).await;
    println!("transferTokens {:?}", res);

    //contract.call("transferTokens", (), my_addr, Default::default())
}