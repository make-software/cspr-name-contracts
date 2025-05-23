import { BigNumberish } from "@ethersproject/bignumber"

import { Args, CLTypeUInt8, CLValue, ContractCallBuilder, PublicKey, Transaction } from "casper-js-sdk"

import { PaymentVoucher, RenewalPaymentVoucher } from "./types";

// eslint-disable-next-line import/prefer-default-export
export class Controller {
  constructor(
    private readonly networkName: string,
    private readonly contractHash: string,
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
    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('buy')
      .runtimeArgs(this.voucherToArgs(voucher, signature))
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
    return new ContractCallBuilder()
      .chainName(this.networkName)
      .from(sender)
      .payment(Number(paymentAmount))
      .byHash(this.contractHash)
      .entryPoint('renew')
      .runtimeArgs(this.voucherToArgs(voucher, signature))
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
      .byHash(this.contractHash)
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
