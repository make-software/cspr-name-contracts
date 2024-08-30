import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLAccountHash, CLBool, CLByteArray, CLKey, CLPublicKey, Contracts, decodeBase16, DeployUtil, RuntimeArgs } from "casper-js-sdk"


// eslint-disable-next-line import/prefer-default-export
export class NameToken {
  private readonly contractClient: Contracts.Contract;

  constructor(
    private readonly networkName: string,
    contractHash: string,
  ) {
    this.contractClient = new Contracts.Contract();

    this.contractClient.setContractHash(`hash-${contractHash}`);
  }

  /**
   * Sets default CSPR.name resolver contract
   * @param resolverContractHash address of the resolver contract
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public setDefaultResolver(
    resolverContractHash: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const runtimeArgs = RuntimeArgs.fromMap({
      resolver: new CLKey(new CLByteArray(decodeBase16(resolverContractHash))),
    });

    return this.contractClient.callEntrypoint(
      'set_default_resolver',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Sets approval for all CSPR.name tokens
   * @param approveAll approval flag
   * @param operator approved operator address
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public setApprovalForAll(
    approveAll: boolean,
    operator: string,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const runtimeArgs = RuntimeArgs.fromMap({
      approve_all: new CLBool(approveAll),
      operator: new CLKey(new CLAccountHash(decodeBase16(operator))),
    });

    return this.contractClient.callEntrypoint(
      'set_approval_for_all',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }
}
