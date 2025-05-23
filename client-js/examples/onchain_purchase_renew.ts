import fs from "fs";

import { HttpHandler, KeyAlgorithm, PrivateKey, RpcClient } from "casper-js-sdk";

import { Controller } from "../src/controller";
import { PaymentInfo, RenewalPaymentVoucher, TokenRenewalInfo } from "../src/types";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminPrivateKeyPem = fs.readFileSync(config.adminPrivateKeyPath, "utf8");
  const adminKeypair = PrivateKey.fromPem(
    adminPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const buyerPrivateKeyPath = `${config.buyerPrivateKeyPath}`;
  const buyerPrivateKeyPem = fs.readFileSync(buyerPrivateKeyPath, "utf8");

  const buyerKeypair = PrivateKey.fromPem(
    buyerPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const expiration = new Date();
  expiration.setFullYear(expiration.getFullYear() + 1, expiration.getMonth(), expiration.getDate());

  const voucher = new RenewalPaymentVoucher(
    new PaymentInfo(buyerKeypair.publicKey.accountHash().toPrefixedString(), "payment:1", 1000),
    [new TokenRenewalInfo(config.mintingName, expiration)],
    expiration,
  );

  const signature = adminKeypair.signAndAddAlgorithmBytes(voucher.toBytes());

  const controllerContract = new Controller(
    config.networkName,
    config.controllerContractHash,
  );

  const transaction = controllerContract.renew(
    voucher,
    signature,
    80000000000,
    buyerKeypair.publicKey,
  );

  transaction.sign(buyerKeypair);

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
