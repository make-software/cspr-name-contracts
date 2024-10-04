import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLPublicKey, CLString, Contracts, DeployUtil, RuntimeArgs } from "casper-js-sdk"


// eslint-disable-next-line import/prefer-default-export
export class ReverseResolver {
  private readonly contractClient: Contracts.Contract;

  constructor(
    private readonly networkName: string,
    contractHash: string,
  ) {
    this.contractClient = new Contracts.Contract();

    this.contractClient.setContractHash(`hash-${contractHash}`);
  }

  /**
   * Sets resolution for a given CSPR.name
   * @param fullDomain full domain in a format of (cname.sld.cspr, sld.cspr)
   * @param address address of the account/contract of cspr.name resolution
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public setPrimaryName(
    primaryName: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const domains = primaryName.split('.')
    if (domains.length < 2) {
      throw new Error('invalid fullDomain format, should be (cname.sld.cspr, sld.cspr)')
    }

    const tld = domains[domains.length - 1]
    if (tld !== 'cspr') {
      throw new Error('top level domain should be equal to .cspr')
    }

    const runtimeArgs = RuntimeArgs.fromMap({
      primary_name: new CLString(primaryName),
    })

    return this.contractClient.callEntrypoint(
      'set_primary_name',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    )
  }
}
