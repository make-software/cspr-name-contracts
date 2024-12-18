import {encodeBase16, Keys } from "casper-js-sdk";

import { Controller } from "../src/controller";
import { NameMintInfo, PaymentInfo, PaymentVoucher } from "../src/types";
import { waitForDeploy } from "./common";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    `${config.adminPrivateKeyPath}`,
  );

  const buyerKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    `${config.adminPrivateKeyPath}`,
  );

  const expiration = new Date();
  expiration.setFullYear(new Date().getFullYear() + 1, 1, 1);
  
  const voucher = new PaymentVoucher(
    new PaymentInfo(buyerKeypair.accountHex(), "payment:1", 10000),
    [new NameMintInfo("alex", encodeBase16(buyerKeypair.accountHash()), expiration)],
    expiration,
  );

  const signature = adminKeypair.sign(voucher.toBytes());

  const controllerContract = new Controller(
    config.networkName,
    config.controllerContractHash,
  );

  const deploy = controllerContract.buy(
    voucher,
    signature,
    10000,
    buyerKeypair.publicKey,
  );

  const signedDeploy = deploy.sign([buyerKeypair]);

  const deployHash = await signedDeploy.send(config.nodeAddress);

  // eslint-disable-next-line no-console
  console.log(`Onchain Buy CSPR.name deploy_hash: ${deployHash}`);

  await waitForDeploy(config.nodeAddress, deployHash);
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
