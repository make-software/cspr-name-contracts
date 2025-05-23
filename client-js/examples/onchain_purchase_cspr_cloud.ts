import fs from "fs";
import { join } from "path";

import { HttpHandler, KeyAlgorithm, PrivateKey, RpcClient } from "casper-js-sdk";

import { Controller } from "../src/controller";
import { NameMintInfo, PaymentInfo, PaymentVoucher } from "../src/types";
import { config } from "./config";
import axios, {AxiosResponse} from "axios";
import {hexToBytes} from "@noble/hashes/utils";

// eslint-disable-next-line @typescript-eslint/require-await
const run = async () => {
  const buyerPrivateKeyPem = fs.readFileSync(config.buyerPrivateKeyPath, "utf8");
  const buyerKeypair = PrivateKey.fromPem(
    buyerPrivateKeyPem,
    KeyAlgorithm.ED25519,
  );

  const csprCloudClient = axios.create({
    baseURL: "https://cspr-name-api.dev.make.services",
  });

  const domain = "sld321";

  const createVoucherResponse = await csprCloudClient.post<any, AxiosResponse<{
    data: {
      voucher: {
        payment_info: {
          buyer: string,
          payment_id: string,
          amount: number
        },
        names: [
          {
            label: string,
            owner: string,
            token_expiration: string,
          }
        ],
        voucher_expiration: string,
      },
      signature: string,
    }
  }>>(`/domains/${domain}/payment-vouchers`, {
    buyer_key: buyerKeypair.publicKey.accountHash().toPrefixedString(),
    annual_periods: 1,
  });

  const createVoucherResponseData = createVoucherResponse.data.data;

  const voucher = new PaymentVoucher(
    new PaymentInfo(
      createVoucherResponseData.voucher.payment_info.buyer,
      createVoucherResponseData.voucher.payment_info.payment_id,
      createVoucherResponseData.voucher.payment_info.amount,
    ),
    createVoucherResponseData.voucher.names.map<NameMintInfo>(n => new NameMintInfo(
      n.label,
      n.owner,
      new Date(n.token_expiration),
    )),
    new Date(createVoucherResponseData.voucher.voucher_expiration),
  );

  const proxyCallerWasmBytes = fs.readFileSync(join(__dirname, 'proxy_caller.wasm'));

  const controllerContract = new Controller(
    config.networkName,
    config.controllerContractPackageHash,
    proxyCallerWasmBytes,
  );

  const transaction = controllerContract.buy(
    voucher,
    hexToBytes(createVoucherResponse.data.data.signature),
    20000000000,
    buyerKeypair.publicKey,
  );

  transaction.sign(buyerKeypair);

  const rpcHandler = new HttpHandler(config.nodeAddress);
  const rpcClient = new RpcClient(rpcHandler);

  try {
    const putTransactionResult = await rpcClient.putTransaction(transaction);

    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    console.log({transactionHash: putTransactionResult.transactionHash.toHex(), result: putTransactionResult.rawJSON});
  } catch(err) {
    console.log({err: err.sourceErr.data});
  }
};

// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access,@typescript-eslint/restrict-template-expressions
run().then(_ => console.log('Finished')).catch(e => console.error(`Error: ${e.stack}`));
