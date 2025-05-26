import * as fs from "fs";

import { HttpHandler, KeyAlgorithm, PrivateKey, RpcClient } from "casper-js-sdk";

import { Controller } from "../src/controller";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminPrivateKeyPem = fs.readFileSync(config.adminPrivateKeyPath, "utf8");
  const adminKeypair = PrivateKey.fromPem(
    adminPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const controllerContract = new Controller(
    config.networkName,
    config.controllerContractHash,
  );

  const transaction = controllerContract.setSignerPublicKey(
    adminKeypair.publicKey,
    adminKeypair.publicKey,
    5000000000,
  );

  transaction.sign(adminKeypair);

  console.log({trx: JSON.stringify(transaction, null, 2)});

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
