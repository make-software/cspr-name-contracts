import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLList, CLPublicKey, CLStringType, Contracts, DeployUtil, RuntimeArgs } from "casper-js-sdk"

import { RenewalVoucher, TokenizationVoucher  } from "./types";

// eslint-disable-next-line import/prefer-default-export
export class Registrar {
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
   * @param voucher @see {@link TokenizationVoucher} TokenizationVoucher that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public register(
    voucher: TokenizationVoucher,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ voucher });

    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for voucher
      voucher: new CLList(new CLStringType()),
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
   * @param voucher @see {@link RenewalVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public prolong(
    voucher: RenewalVoucher,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ voucher });

    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for voucher
      voucher: new CLList(new CLStringType()),
    });

    return this.contractClient.callEntrypoint(
      'prolong',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }
}
