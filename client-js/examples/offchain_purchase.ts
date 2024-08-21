import { encodeBase16, Keys } from "casper-js-sdk";

import { Registrar } from "../src/registrar";
import { TokenizationVoucher } from "../src/types";
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

  const voucher: TokenizationVoucher = {
    names: [{
      // sld.cspr
      label: "sld",
      owner: encodeBase16(buyerKeypair.accountHash()),
      token_expiration: expiration,
    }],
    voucher_expiration: expiration,
  };

  const registrarContract = new Registrar(
    config.networkName,
    config.controllerContractHash,
  );

  const deploy = registrarContract.register(
    voucher,
    10000,
    adminKeypair.publicKey,
  );

  const signedDeploy = deploy.sign([adminKeypair]);

  const deployHash = await signedDeploy.send(config.nodeAddress);

  // eslint-disable-next-line no-console
  console.log(`Offnchain Register CSPR.name deploy_hash: ${deployHash}`);

  await waitForDeploy(config.nodeAddress, deployHash);
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
