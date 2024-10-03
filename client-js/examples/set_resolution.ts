import { CasperClient, encodeBase16, Keys } from "casper-js-sdk";

import { DefaultResolver } from "../src/default-resolver";
import { config } from "./config";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const ownerKeypair = Keys.Ed25519.loadKeyPairFromPrivateFile(
    config.adminPrivateKeyPath,
  );

  const defaultResolver = new DefaultResolver(
    config.networkName,
    config.defaultResolverContractHash,
  );

  const deploy = defaultResolver.setResolution(
    'name.sld1.cspr',
    encodeBase16(ownerKeypair.accountHash()),
    // 20 CSPR
    20000000000,
    ownerKeypair.publicKey,
  );

  const client = new CasperClient(config.nodeAddress);

  const signedDeploy = client.signDeploy(deploy, ownerKeypair);

  console.log(`DEPLOY_HASH: ${encodeBase16(deploy.hash)}`);

  const deployHash = await signedDeploy.send(config.nodeAddress)

  // eslint-disable-next-line no-console
  console.log(`Offnchain Register CSPR.name deploy_hash: ${deployHash}`)
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();
