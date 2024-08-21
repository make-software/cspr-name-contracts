import {Keys } from "casper-js-sdk";

import { Controller } from "../src/controller";
import { PaymentInfo, RenewalPaymentVoucher, TokenRenewalInfo } from "../src/types";
import { waitForDeploy } from "./common";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminKeypair = Keys.Ed25519.parseKeyFiles(
    `${config.adminPrivateKeypairPath}/public_key.pem`,
    `${config.adminPrivateKeypairPath}/secret_key.pem`,
  );

  const buyerKeypair = Keys.Ed25519.parseKeyFiles(
    `${config.adminPrivateKeypairPath}/public_key.pem`,
    `${config.adminPrivateKeypairPath}/secret_key.pem`,
  );

  const expiration = new Date();
  expiration.setFullYear(new Date().getFullYear() + 1, 1, 1);

  const voucher = new RenewalPaymentVoucher(
    new PaymentInfo(buyerKeypair.accountHex(), "payment:1", 10000),
    [new TokenRenewalInfo("some-hash", expiration)],
    expiration,
  );

  const controllerContract = new Controller(
    config.networkName,
    config.controllerContractHash,
  );

  const signature = adminKeypair.sign(voucher.toBytes());

  const deploy = controllerContract.renew(
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
