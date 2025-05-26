import fs from "fs";

import { HttpHandler, KeyAlgorithm, PrivateKey, RpcClient } from "casper-js-sdk";

import { DefaultResolver } from "../src/default-resolver";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const ownerPrivateKeyPem = fs.readFileSync(config.adminPrivateKeyPath, "utf8");
  const ownerKeypair = PrivateKey.fromPem(
    ownerPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const defaultResolver = new DefaultResolver(
    config.networkName,
    config.defaultResolverContractPackageHash,
  );

  const transaction = defaultResolver.setResolution(
    config.mintingName,
    ownerKeypair.publicKey.accountHash().toPrefixedString(),
    // 20 CSPR
    20000000000,
    ownerKeypair.publicKey,
  );

  transaction.sign(ownerKeypair);

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
