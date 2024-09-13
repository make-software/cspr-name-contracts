import { Keys } from "casper-js-sdk";

import { Registrar } from "../src/registrar";
import { TokenRenewalInfo } from "../src/types";
import { waitForDeploy } from "./common";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    `${config.adminPrivateKeyPath}/secret_key.pem`,
  );

  const registrarContract = new Registrar(
    config.networkName,
    config.registrarContractHash,
  );

  const expiration = new Date();
  expiration.setFullYear(new Date().getFullYear() + 1, 1, 1);

  const deploy = registrarContract.adminProlong(
    [new TokenRenewalInfo("some-hash", expiration)],
    10000,
    adminKeypair.publicKey,
  );

  const signedDeploy = deploy.sign([adminKeypair]);

  const deployHash = await signedDeploy.send(config.nodeAddress);

  // eslint-disable-next-line no-console
  console.log(`Offchain Prolong CSPR.name deploy_hash: ${deployHash}`);

  await waitForDeploy(config.nodeAddress, deployHash);
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
