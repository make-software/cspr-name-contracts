import { CasperClient, encodeBase16, Keys } from "casper-js-sdk";

import { Registrar } from "../src/registrar";
import { NameMintInfo } from "../src/types";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const adminKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    config.adminPrivateKeyPath,
  );

  const buyerKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    config.adminPrivateKeyPath,
  );

  const expiration = new Date();
  expiration.setFullYear(new Date().getFullYear() + 1, 1, 1);

  const nameMintInfos = [
    new NameMintInfo("sld1", encodeBase16(buyerKeypair.accountHash()), expiration),
  ]

  const registrarContract = new Registrar(
    config.networkName,
    config.registrarContractHash,
  );

  const deploy = registrarContract.adminRegister(
    nameMintInfos,
    // 20 CSPR
    20000000000,
    adminKeypair.publicKey,
  );

  const client = new CasperClient(config.nodeAddress);

  const signedDeploy = client.signDeploy(deploy, adminKeypair);

  console.log(`DEPLOY_HASH: ${encodeBase16(deploy.hash)}`);

  const deployHash = await signedDeploy.send(config.nodeAddress)

  // eslint-disable-next-line no-console
  console.log(`Offnchain Register CSPR.name deploy_hash: ${deployHash}`)
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
