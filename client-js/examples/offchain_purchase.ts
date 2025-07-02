import fs from "fs";

import { HttpHandler, KeyAlgorithm, PrivateKey, RpcClient } from "casper-js-sdk";

import { Registrar } from "../src/registrar";
import { NameMintInfo } from "../src/types";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminPrivateKeyPem = fs.readFileSync(config.adminPrivateKeyPath, "utf8");

  const adminKeypair = PrivateKey.fromPem(
    adminPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const buyerPrivateKeyPem = fs.readFileSync(config.buyerPrivateKeyPath, "utf8");
  const buyerKeypair = PrivateKey.fromPem(
    buyerPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const expiration = new Date();
  expiration.setFullYear(expiration.getFullYear() + 1, expiration.getMonth(), expiration.getDate() + 1);

  const nameMintInfos = [
    new NameMintInfo(config.mintingName, buyerKeypair.publicKey.accountHash().toPrefixedString(), expiration, config.nameTokenURL),
  ];

  const registrarContract = new Registrar(
    config.networkName,
    config.registrarContractPackageHash,
  );

  const transaction = registrarContract.adminRegister(
    nameMintInfos,
    // 20 CSPR
    20000000000,
    adminKeypair.publicKey,
  );

  transaction.sign(adminKeypair);

  const rpcHandler = new HttpHandler(config.nodeAddress);
  const rpcClient = new RpcClient(rpcHandler);

  try {
    const putTransactionResult = await rpcClient.putTransaction(transaction);

    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    console.log({transactionHash: putTransactionResult.transactionHash.toHex(), result: putTransactionResult.rawJSON});
  } catch(err) {
    console.log({err: err.sourceErr.data});
  }
};

// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access,@typescript-eslint/restrict-template-expressions
run().then(_ => console.log('Finished')).catch(e => console.error(`Error: ${e.stack}`));
