import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLList, CLPublicKey, CLString, CLStringType, Contracts, DeployUtil, Keys, RuntimeArgs } from "casper-js-sdk"

import { PaymentInfo, PaymentVaucher, RenewalPaymentVaucher } from "./types";

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
   * @param vaucher @see {@link PaymentVaucher} PaymentVaucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVaucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public buy(
    vaucher: PaymentVaucher,
    signature: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ vaucher });
    
    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for vaucher
      vaucher: new CLList(new CLStringType()),
      signature: new CLString(signature),
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
   * @param vaucher @see {@link RenewalPaymentVaucher} PaymentVaucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVaucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public renew(
    vaucher: RenewalPaymentVaucher,
    signature: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ vaucher });

    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for vaucher
      vaucher: new CLList(new CLStringType()),
      signature: new CLString(signature),
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
