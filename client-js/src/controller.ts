import { BigNumberish } from "@ethersproject/bignumber"

import { Args, CLTypeUInt8, CLValue, ContractCallBuilder, PublicKey, SessionBuilder, Transaction } from "casper-js-sdk"

import { PaymentVoucher, RenewalPaymentVoucher } from "./types";
import {hexToBytes} from "@noble/hashes/utils";

// eslint-disable-next-line import/prefer-default-export
export class Controller {
  constructor(
    private readonly networkName: string,
    private readonly contractPackageHash: string,
    private readonly proxyCallWasm: Uint8Array,
  ) {}

  /**
   * Buys CSPR.name for an account
   * @param voucher @see {@link PaymentVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider) with algorithm bytes included
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public buy(
    voucher: PaymentVoucher,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const rawArgsBytes = this.voucherToArgs(voucher, signature).toBytes();
    const argsBytes: CLValue[] = [];
    for (let i = 0; i < rawArgsBytes.length; i += 1) {
      argsBytes.push(CLValue.newCLUint8(rawArgsBytes[i]));
    }

    return new SessionBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .wasm(this.proxyCallWasm)
      .runtimeArgs(Args.fromMap({
        package_hash: CLValue.newCLByteArray(hexToBytes(this.contractPackageHash)),
        entry_point: CLValue.newCLString("buy"),
        args: CLValue.newCLList(CLTypeUInt8, argsBytes),
        amount: CLValue.newCLUInt512(voucher.payment.amount),
        attached_value: CLValue.newCLUInt512(voucher.payment.amount),
      }))
      .build();
  }

  /**
   * Buys CSPR.name for an account
   * @param voucher @see {@link RenewalPaymentVoucher} PaymentVoucher that was created by CSPR.name provider
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public renew(
    voucher: RenewalPaymentVoucher,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const rawArgsBytes = this.voucherToArgs(voucher, signature).toBytes();
    const argsBytes: CLValue[] = [];
    for (let i = 0; i < rawArgsBytes.length; i += 1) {
      argsBytes.push(CLValue.newCLUint8(rawArgsBytes[i]));
    }

    return new SessionBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .wasm(this.proxyCallWasm)
      .runtimeArgs(Args.fromMap({
        package_hash: CLValue.newCLByteArray(hexToBytes(this.contractPackageHash)),
        entry_point: CLValue.newCLString("renew"),
        args: CLValue.newCLList(CLTypeUInt8, argsBytes),
        amount: CLValue.newCLUInt512(voucher.payment.amount),
        attached_value: CLValue.newCLUInt512(voucher.payment.amount),
      }))
      .build();
  }

  /**
   * Sets public key of a voucher signer account
   * @param sender the PublicKey of transaction submitter account
   * @param signer the PublicKey of a new voucher signer account
   * @param paymentAmount the amount of gas price that should be paid in motes

   * @returns Transaction object which can be sent to the node.
   */
  public setSignerPublicKey(
    sender: PublicKey,
    signer: PublicKey,
    paymentAmount: BigNumberish,
  ): Transaction {
    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractPackageHash)
      .entryPoint('set_signer_public_key')
      .runtimeArgs(Args.fromMap({
        signer: CLValue.newCLPublicKey(signer),
      }))
      .build();
  }

  // eslint-disable-next-line class-methods-use-this
  private voucherToArgs(voucher: { toBytes(): Uint8Array }, signature: Uint8Array): Args {
    const rawVoucherBytes = voucher.toBytes();

    const signatureBytes: CLValue[] = [];
    for (let i = 0; i < signature.length; i += 1) {
      signatureBytes.push(CLValue.newCLUint8(signature[i]));
    }

    return Args.fromMap({
      voucher: CLValue.newCLAny(rawVoucherBytes),
      signature: CLValue.newCLList(CLTypeUInt8, signatureBytes),
    });
  }
}
