import { BigNumberish } from "@ethersproject/bignumber"
import {
  Args,
  CLValue,
  ContractCallBuilder, Key, PublicKey, Transaction,
} from "casper-js-sdk"

// eslint-disable-next-line import/prefer-default-export
export class DefaultResolver {
  constructor(
    private readonly networkName: string,
    private readonly contractPackageHash: string,
  ) {}

  /**
   * Sets resolution for a given CSPR.name
   * @param fullDomain full domain in a format of (cname.sld.cspr, sld.cspr)
   * @param address address of the account/contract of cspr.name resolution (starts with either account-hash-... or hash-...)
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public setResolution(
    fullDomain: string,
    address: string,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const domains = fullDomain.split('.')
    if (domains.length < 2) {
      throw new Error('invalid fullDomain format, should be (cname.sld.cspr, sld.cspr)')
    }

    const tld = domains[domains.length - 1]
    if (tld !== 'cspr') {
      throw new Error('top level domain should be equal to .cspr')
    }

    const addressKey = CLValue.newCLKey(Key.newKey(address));

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byPackageHash(this.contractPackageHash)
      .entryPoint('set_resolution')
      .runtimeArgs(Args.fromMap({
        full_domain: CLValue.newCLString(fullDomain),
        address: CLValue.newCLOption(addressKey, addressKey.type),
      }))
      .build()
  }
}
