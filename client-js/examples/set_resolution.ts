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

  // address - encodeBase16(ownerKeypair.publicKey.toAccountHash())

  const deploy = defaultResolver.setResolution(
    'wallet-extension-ledger-test-name.cspr',
    'd4497e8d5f6e1565f5be6964be7da3f48523e307adc829858071cdd5e981d425',
    // 2 CSPR
    2000000000,
    ownerKeypair.publicKey,
  );

  const client = new CasperClient(config.nodeAddress);

  const signedDeploy = client.signDeploy(deploy, ownerKeypair);

  console.log(`DEPLOY_HASH: ${encodeBase16(deploy.hash)}`);

  const deployHash = await signedDeploy.send(config.nodeAddress)

  // eslint-disable-next-line no-console
  console.log(`Set Resolution for CSPR.name deploy_hash: ${deployHash}`)
};

// eslint-disable-next-line @typescript-eslint/no-floating-promises
run();

// My account names

// vova.cspr     -> alex account
// maketeam.cspr -> my account
// dev.cspr      -> ihor account

// vova.cspr name resolutions
// vova.cspr     -> alex account
// dev.vova.cspr -> my account
// team.dev.vova.cspr -> other account

// Alex's account names

// alex.cspr     -> alex account

// Assigned to alex names

// alex.cspr -> alex account
// vova.cspr -> alex account
