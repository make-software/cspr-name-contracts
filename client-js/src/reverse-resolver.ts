import { BigNumberish } from "@ethersproject/bignumber"
import {
  Args,
  CLValue,
  ContractCallBuilder, PublicKey, Transaction,
} from "casper-js-sdk"


// eslint-disable-next-line import/prefer-default-export
export class ReverseResolver {
  constructor(
    private readonly networkName: string,
    private readonly contractPackageHash: string,
  ) {}

  /**
   * Sets resolution for a given CSPR.name
   * @param primaryName full domain in a format of (cname.sld.cspr, sld.cspr)
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public setPrimaryName(
    primaryName: string,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const domains = primaryName.split('.')
    if (domains.length < 2) {
      throw new Error('invalid primary name format, should be (cname.sld.cspr, sld.cspr)')
    }

    const tld = domains[domains.length - 1]
    if (tld !== 'cspr') {
      throw new Error('top level domain should be equal to .cspr')
    }

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byPackageHash(this.contractPackageHash)
      .entryPoint('set_primary_name')
      .runtimeArgs(Args.fromMap({
        primary_name: CLValue.newCLString(primaryName),
      }))
      .build();
  }
}
