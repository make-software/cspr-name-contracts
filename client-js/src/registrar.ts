import { BigNumberish } from "@ethersproject/bignumber"
import {
  ContractCallBuilder,
  PublicKey,
  Transaction, CLValue, CLTypeUInt8, CLTypeString, Key, Args, toBytesU32
} from "casper-js-sdk"

import { NameMintInfo, TokenRenewalInfo } from "./types";

// eslint-disable-next-line import/prefer-default-export
export class Registrar {
  constructor(
    private readonly networkName: string,
    private readonly contractHash: string,
  ) {}

  /**
   * Buys CSPR.name for an account
   * @param nameMintInfos @see {@link NameMintInfo} NameMintInfo[] that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public adminRegister(
    nameMintInfos: NameMintInfo[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const length = toBytesU32(nameMintInfos.length)

    let bytes = Array.from(length)
    bytes = nameMintInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    const runtimeArgs = Args.fromMap({
        names: CLValue.newCLAny(Uint8Array.from(bytes)),
    })

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('admin_register')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Buys CSPR.name for an account
   * @param tokenRenewalInfos @see {@link TokenRenewalInfo} TokenRenewalInfo[] that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public adminProlong(
    tokenRenewalInfos: TokenRenewalInfo[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const length = toBytesU32(tokenRenewalInfos.length)

    let bytes = Array.from(length)
    bytes = tokenRenewalInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    const infosBytes: CLValue[] = []
    for (let i = 0; i < bytes.length; i += 1) {
      infosBytes.push(CLValue.newCLUint8(bytes[ i]));
    }

    const runtimeArgs = Args.fromMap({
      tokens: CLValue.newCLList(CLTypeUInt8, infosBytes),
    })

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('admin_prolong')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Buys CSPR.name for an account
   * @param tokenRenewalInfos @see {@link TokenRenewalInfo} TokenRenewalInfo[] that was created by CSPR.name provider
   * @param nameMintInfos @see {@link NameMintInfo} NameMintInfo[] that was created by CSPR.name provider
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public adminProlongAndRegister(
    tokenRenewalInfos: TokenRenewalInfo[],
    nameMintInfos: NameMintInfo[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const tokenRenewalInfosLength = toBytesU32(tokenRenewalInfos.length)

    let tokenRenewalBytes = Array.from(tokenRenewalInfosLength)
    tokenRenewalBytes = tokenRenewalInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), tokenRenewalBytes)
    const tokenRenewalCLBytes: CLValue[] = []
    for (let i = 0; i < tokenRenewalBytes.length; i += 1) {
      tokenRenewalCLBytes.push(CLValue.newCLUint8(tokenRenewalBytes[ i]));
    }

    const nameMintInfosLength = toBytesU32(nameMintInfos.length)

    let nameMintInfosBytes = Array.from(nameMintInfosLength)
    nameMintInfosBytes = nameMintInfos.reduce((b, name) => b.concat(Array.from(name.toBytes())), nameMintInfosBytes)
    const nameMintInfosCLBytes: CLValue[] = []
    for (let i = 0; i < tokenRenewalBytes.length; i += 1) {
      nameMintInfosCLBytes.push(CLValue.newCLUint8(nameMintInfosBytes[ i]));
    }

    const runtimeArgs = Args.fromMap({
      renewal_tokens: CLValue.newCLList(CLTypeUInt8, tokenRenewalCLBytes),
      new_tokens: CLValue.newCLList(CLTypeUInt8, nameMintInfosCLBytes),
    })

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('admin_prolong_and_register')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Sets grace period of name token for registrar contract as an admin
   * @param periodMilliseconds grace period in milliseconds
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public setGracePeriod(
    periodMilliseconds: number,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const runtimeArgs = Args.fromMap({
      period: CLValue.newCLUint64(periodMilliseconds),
    });

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('set_grace_period')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Transfer CSPR.name to an account as an admin
   * @param newOwnerHash hash of the new name token owner
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public adminTransfer(
    newOwnerHash: string,
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const tokenHashesList = tokenHashes.map(th => CLValue.newCLString(th));
    const runtimeArgs = Args.fromMap({
      new_owner: CLValue.newCLKey(Key.newKey(newOwnerHash)),
      token_hashes: CLValue.newCLList(CLTypeString, tokenHashesList),
    });

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('admin_transfer')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Burn CSPR.name tokens as an admin
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public adminBurn(
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const tokenHashesList = tokenHashes.map(th => CLValue.newCLString(th));
    const runtimeArgs = Args.fromMap({
      token_hashes: CLValue.newCLList(CLTypeString, tokenHashesList),
    });

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('admin_burn')
      .runtimeArgs(runtimeArgs)
      .build();
  }

  /**
   * Expire CSPR.name tokens as an admin
   * @param tokenHashes list of token hashes for transfer
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account (admin)
   * @returns Transaction object which can be sent to the node.
   */
  public expire(
    tokenHashes: string[],
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const tokenHashesList = tokenHashes.map(th => CLValue.newCLString(th));
    const runtimeArgs = Args.fromMap({
      token_hashes: CLValue.newCLList(CLTypeString, tokenHashesList),
    });

    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('expire')
      .runtimeArgs(runtimeArgs)
      .build();
  }
}
