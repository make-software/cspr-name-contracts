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
    new NameMintInfo("sld", buyerKeypair.accountHex(), expiration),
  ]

  const registrarContract = new Registrar(
    config.networkName,
    config.registrarContractHash,
  );

  const deploy = registrarContract.adminRegister(
    nameMintInfos,
    // 10 CSPR
    10000000000,
    adminKeypair.publicKey,
  );

  const client = new CasperClient(config.nodeAddress);

  console.log({ config })

  const signedDeploy = client.signDeploy(deploy, adminKeypair);

  console.log(`DEPLOY_HASH: ${encodeBase16(deploy.hash)}`);
  

  console.log({ config })


  const deployHash = await signedDeploy.send(config.nodeAddress)

  console.log({ config })

  // eslint-disable-next-line no-console
  console.log(`Offnchain Register CSPR.name deploy_hash: ${deployHash}`)
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
