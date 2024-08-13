import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLList, CLPublicKey, CLStringType, Contracts, DeployUtil, RuntimeArgs } from "casper-js-sdk"

import { RenewalVaucher, TokenizationVaucher  } from "./types";

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
   * @param vaucher @see {@link TokenizationVaucher} TokenizationVaucher that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public register(
    vaucher: TokenizationVaucher,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ vaucher });

    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for vaucher
      vaucher: new CLList(new CLStringType()),
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
   * @param vaucher @see {@link RenewalVaucher} PaymentVaucher that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public prolong(
    vaucher: RenewalVaucher,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    // eslint-disable-next-line no-console
    console.log({ vaucher });

    const runtimeArgs = RuntimeArgs.fromMap({
      // TODO: Implement serialisation for vaucher
      vaucher: new CLList(new CLStringType()),
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
