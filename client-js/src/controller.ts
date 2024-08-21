import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLList, CLPublicKey, CLU8, Contracts, DeployUtil, Keys, RuntimeArgs } from "casper-js-sdk"

import { PaymentInfo, PaymentVoucher, RenewalPaymentVoucher } from "./types";

// eslint-disable-next-line import/prefer-default-export
export class Controller {
  private readonly contractClient: Contracts.Contract;

  constructor(
    private readonly networkName: string,
    contractHash: string,
  ) {
    this.contractClient = new Contracts.Contract();

    this.contractClient.setContractHash(`hash-${contractHash}`);
  }

  /**
   * Buys CSPR.name for an account
   * @param voucher @see {@link PaymentVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public buy(
    voucher: PaymentVoucher,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const rawVoucherBytes = voucher.toBytes();

    const voucherBytes: CLU8[] = [];
    for (let i = 0; i < rawVoucherBytes.length; i+=1) {
      voucherBytes.push(new CLU8(rawVoucherBytes[i]));
    }

    const signatureBytes: CLU8[] = [];
    for (let i = 0; i < signature.length; i += 1) {
      signatureBytes.push(new CLU8(signature[i]));
    }
    
    const runtimeArgs = RuntimeArgs.fromMap({
      voucher: new CLList(voucherBytes),
      signature: new CLList(signatureBytes),
    });

    return this.contractClient.callEntrypoint(
      'buy',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Buys CSPR.name for an account
   * @param voucher @see {@link RenewalPaymentVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public renew(
    voucher: RenewalPaymentVoucher,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const rawVoucherBytes = voucher.toBytes();

    const voucherBytes: CLU8[] = [];
    for (let i = 0; i < rawVoucherBytes.length; i += 1) {
      voucherBytes.push(new CLU8(rawVoucherBytes[i]));
    }

    const signatureBytes: CLU8[] = [];
    for (let i = 0; i < signature.length; i += 1) {
      signatureBytes.push(new CLU8(signature[i]));
    }

    const runtimeArgs = RuntimeArgs.fromMap({
      voucher: new CLList(voucherBytes),
      signature: new CLList(signatureBytes),
    });

    return this.contractClient.callEntrypoint(
      'renew',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  // eslint-disable-next-line class-methods-use-this, unused-imports/no-unused-vars
  public signPaymentInfo(paymentInfo: PaymentInfo, signingKey: Keys.AsymmetricKey): string {
    // TODO: Implement signing for PaymentInfo
    return ""
  }
}
