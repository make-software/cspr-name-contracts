import { BigNumberish } from "@ethersproject/bignumber"
import {
  Args, CLValue, ContractCallBuilder, Key,
  PublicKey, Transaction,
} from "casper-js-sdk"

// eslint-disable-next-line import/prefer-default-export
export class NameToken {
  constructor(
    private readonly networkName: string,
    private readonly contractHash: string,
  ) {}

  /**
   * Sets default CSPR.name resolver contract
   * @param resolverContractHash address of the resolver contract
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public setDefaultResolver(
    resolverContractHash: string,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('set_default_resolver')
      .runtimeArgs(Args.fromMap({
        resolver: CLValue.newCLKey(Key.newKey(resolverContractHash)),
      }))
      .build();
  }

  /**
   * Sets approval for all CSPR.name tokens
   * @param approveAll approval flag
   * @param operator approved operator address (starts with either account-hash-... or hash-...)
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public setApprovalForAll(
    approveAll: boolean,
    operator: string,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('set_approval_for_all')
      .runtimeArgs(Args.fromMap({
        approve_all: CLValue.newCLValueBool(approveAll),
        operator: CLValue.newCLKey(Key.newKey(operator))
      }))
      .build();
  }
}
