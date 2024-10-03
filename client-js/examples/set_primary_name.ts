import { CasperClient, encodeBase16, Keys } from "casper-js-sdk";

import { ReverseResolver } from "../src/reverse-resolver";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const ownerKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    config.adminPrivateKeyPath,
  );

  const reverseResolver = new ReverseResolver(
    config.networkName,
    config.reverseResolutionContractHash,
  );

  const deploy = reverseResolver.setPrimaryName(
    'name.sld1.cspr',
    // 20 CSPR
    800000000,
    ownerKeypair.publicKey,
  );

  const client = new CasperClient(config.nodeAddress);

  const signedDeploy = client.signDeploy(deploy, ownerKeypair);

  console.log(`DEPLOY_HASH: ${encodeBase16(deploy.hash)}`);

  const deployHash = await signedDeploy.send(config.nodeAddress)

  // eslint-disable-next-line no-console
  console.log(`Set Primary CSPR.name deploy_hash: ${deployHash}`)
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
