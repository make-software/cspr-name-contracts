import { BigNumber, BigNumberish } from "@ethersproject/bignumber"
import { CLAccountHash,CLKey, CLList, CLPublicKey, CLString, CLU32,CLU32BytesParser, CLU64, Contracts, decodeBase16,DeployUtil,RuntimeArgs } from "casper-js-sdk"

import { CLAny, NameMintInfo, TokenRenewalInfo  } from "./types";

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
   * @param nameMintInfos @see {@link NameMintInfo[]} NameMintInfo[] that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public adminRegister(
    nameMintInfos: NameMintInfo[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const nameMintInfosLength = new CLU32BytesParser().toBytes(new CLU32(nameMintInfos.length)).unwrap()

    let bytes = Array.from(nameMintInfosLength)
    bytes = nameMintInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    const runtimeArgs = RuntimeArgs.fromMap({
      names: new CLAny(Uint8Array.from(bytes))
    })

    return this.contractClient.callEntrypoint(
      'admin_register',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    )
  }

  /**
   * Buys CSPR.name for an account
   * @param voucher @see {@link RenewalVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account
   * @returns Deploy object which can be send to the node.
   */
  public adminProlong(
    tokenRenewalInfos: TokenRenewalInfo[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const tokenRenewalInfosLength = new CLU32BytesParser().toBytes(new CLU32(tokenRenewalInfos.length)).unwrap()

    let bytes = Array.from(tokenRenewalInfosLength)
    bytes = tokenRenewalInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    const runtimeArgs = RuntimeArgs.fromMap({
      tokens: new CLAny(Uint8Array.from(bytes)),
    });

    return this.contractClient.callEntrypoint(
      'admin_prolong',
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
  public adminProlongAndRegister(
    tokenRenewalInfos: TokenRenewalInfo[],
    nameMintInfos: NameMintInfo[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const tokenRenewalInfosLength = new CLU32BytesParser().toBytes(new CLU32(tokenRenewalInfos.length)).unwrap()

    let tokenRenewalBytes = Array.from(tokenRenewalInfosLength)
    tokenRenewalBytes = tokenRenewalInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), tokenRenewalBytes)

    const nameMintInfosLength = new CLU32BytesParser().toBytes(new CLU32(nameMintInfos.length)).unwrap()

    let nameMintInfosBytes = Array.from(nameMintInfosLength)
    nameMintInfosBytes = nameMintInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), nameMintInfosBytes)

    const runtimeArgs = RuntimeArgs.fromMap({
      renewal_tokens: new CLAny(Uint8Array.from(tokenRenewalBytes)),
      new_tokens: new CLAny(Uint8Array.from(nameMintInfosBytes)),
    });

    return this.contractClient.callEntrypoint(
      'admin_prolong_and_register',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Sets grace period of name token for registrar contract as an admin
   * @param periodMilliseconds grace period in milliseconds
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public setGracePeriod(
    periodMilliseconds: number,
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const runtimeArgs = RuntimeArgs.fromMap({
      period: new CLU64(periodMilliseconds),
    });

    return this.contractClient.callEntrypoint(
      'set_grace_period',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Transfer CSPR.name to an account as an admin
   * @param newOwnerAccountHash account hash of the new name token owner
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public adminTransfer(
    newOwnerAccountHash: string,
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const tokenHashesList = tokenHashes.map(th => new CLString(th));
    const runtimeArgs = RuntimeArgs.fromMap({
      new_owner: new CLKey(new CLAccountHash(decodeBase16(newOwnerAccountHash))),
      token_hashes: new CLList(tokenHashesList),
    });

    return this.contractClient.callEntrypoint(
      'admin_transfer',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Burn CSPR.name tokens as an admin
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public adminBurn(
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const tokenHashesList = tokenHashes.map(th => new CLString(th));
    const runtimeArgs = RuntimeArgs.fromMap({
      token_hashes: new CLList(tokenHashesList),
    });

    return this.contractClient.callEntrypoint(
      'admin_burn',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }

  /**
   * Expire CSPR.name tokens as an admin
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be payed in motes
   * @param sender the CLPublicKey of deploy submitter account (admin)
   * @returns Deploy object which can be send to the node.
   */
  public expire(
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: CLPublicKey,
  ): DeployUtil.Deploy {
    const tokenHashesList = tokenHashes.map(th => new CLString(th));
    const runtimeArgs = RuntimeArgs.fromMap({
      token_hashes: new CLList(tokenHashesList),
    });

    return this.contractClient.callEntrypoint(
      'expire',
      runtimeArgs,
      sender,
      this.networkName,
      BigNumber.from(paymentAmount).toString(),
    );
  }
}
