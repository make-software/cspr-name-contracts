import { CasperClient } from "casper-js-sdk";

// eslint-disable-next-line no-promise-executor-return
export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const getDeploy = async (NODE_ADDRESS: string, deployHash: string) => {
  const client = new CasperClient(NODE_ADDRESS);
  let i = 300;
  while (i !== 0) {
    // eslint-disable-next-line no-await-in-loop
    const [deploy, raw] = await client.getDeploy(deployHash);

    if (raw.execution_results.length !== 0) {
      if (raw.execution_results[0].result.Success) {
        return deploy;
      }

      throw Error(
        `Deploy execution: ${raw.execution_results[0].result.Failure.error_message}`
      );
    }

    // eslint-disable-next-line no-plusplus
    i--;
    // eslint-disable-next-line no-await-in-loop
    await sleep(3000);
  }
  throw Error(`Timeout after ${  i  }s. Something's wrong`);
};

export const waitForDeploy = async (
  NODE_ADDRESS: string,
  deployHash: string
) => {
  // eslint-disable-next-line no-console
  console.log(
    `... Contract deploy is pending, waiting for next block finalisation (deployHash: ${deployHash}) ...`
  );

  const deploy = await getDeploy(NODE_ADDRESS, deployHash);
  return deploy;
};
