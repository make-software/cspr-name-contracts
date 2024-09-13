import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLAccountHash, CLKey, CLOption, CLPublicKey, CLString, Contracts, decodeBase16, DeployUtil, RuntimeArgs } from "casper-js-sdk"
import { Some } from 'ts-results'


// eslint-disable-next-line import/prefer-default-export
export class DefaultResolver {
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
  public setResolution(
    fullDomain: string,
    address: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const domains = fullDomain.split('.')
    if (domains.length < 2) {
      throw new Error('invalid fullDomain format, should be (cname.sld.cspr, sld.cspr)')
    }

    const tld = domains[domains.length - 1]
    if (tld !== 'cspr') {
      throw new Error('top level domain should be equal to .cspr')
    }

    const addressKey = new CLKey(new CLAccountHash(decodeBase16(address)))

    const runtimeArgs = RuntimeArgs.fromMap({
      full_domain: new CLString(fullDomain),
      address: new CLOption(Some(addressKey), addressKey.clType())
    })

    return this.contractClient.callEntrypoint(
      'set_resolution',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    )
  }
}
