import { BigNumberish } from "@ethersproject/bignumber"
import { hexToBytes } from "@noble/hashes/utils";
import {
    Args,
    CLTypeUInt8,
    CLValue,
    ContractCallBuilder,
    Key,
    PublicKey,
    SessionBuilder,
    Transaction
} from "casper-js-sdk"

// eslint-disable-next-line import/prefer-default-export
export class Controller {
  constructor(
    private readonly networkName: string,
    private readonly contractPackageHash: string,
    private readonly proxyCallWasm: Uint8Array,
  ) {}

  /**
   * Buys CSPR.name for an account
   * @param voucherBytes Uint8Array of voucher bytes provided by contract admin
   * @param domainFee amount to be payed for the domain
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider) with algorithm bytes included
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public buy(
    voucherBytes: Uint8Array,
    domainFee: BigNumberish,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const signatureBytes: CLValue[] = [];
    for (let i = 0; i < signature.length; i += 1) {
      signatureBytes.push(CLValue.newCLUint8(signature[i]));
    }

    const rawArgsBytes = Args.fromMap({
      voucher: CLValue.newCLAny(voucherBytes),
      signature: CLValue.newCLList(CLTypeUInt8, signatureBytes),
    }).toBytes();

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
        amount: CLValue.newCLUInt512(domainFee),
        attached_value: CLValue.newCLUInt512(domainFee),
      }))
      .build();
  }

  /**
   * Buys CSPR.name for an account
   * @param voucherBytes Uint8Array of voucher bytes provided by contract admin
   * @param domainFee amount to be payed for the domain
   * @param signature signature of the signer of PaymentVoucher (CSPR.name provider)
   * @param paymentAmount the amount of gas price that should be paid in motes
   * @param sender the PublicKey of transaction submitter account
   * @returns Transaction object which can be sent to the node.
   */
  public renew(
    voucherBytes: Uint8Array,
    domainFee: BigNumberish,
    signature: Uint8Array,
    paymentAmount: BigNumberish,
    sender: PublicKey,
  ): Transaction {
    const signatureBytes: CLValue[] = [];
    for (let i = 0; i < signature.length; i += 1) {
      signatureBytes.push(CLValue.newCLUint8(signature[i]));
    }

    const rawArgsBytes = Args.fromMap({
      voucher: CLValue.newCLAny(voucherBytes),
      signature: CLValue.newCLList(CLTypeUInt8, signatureBytes),
    }).toBytes();

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
        amount: CLValue.newCLUInt512(domainFee),
        attached_value: CLValue.newCLUInt512(domainFee),
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
      .byPackageHash(this.contractPackageHash)
      .entryPoint('set_signer_public_key')
      .runtimeArgs(Args.fromMap({
        signer: CLValue.newCLPublicKey(signer),
      }))
      .build();
  }

    /**
     * Sets public key of a voucher signer account
     * @param sender the PublicKey of transaction submitter account
     * @param treasuryAccountHash the Account Hash of a new treasury account
     * @param paymentAmount the amount of gas price that should be paid in motes

     * @returns Transaction object which can be sent to the node.
     */
    public setTreasuryAccountAddress(
        sender: PublicKey,
        treasuryAccountHash: string,
        paymentAmount: BigNumberish,
    ): Transaction {
        return new ContractCallBuilder()
          .chainName(this.networkName)
          .from(sender)
          .payment(Number(paymentAmount))
          .byPackageHash(this.contractPackageHash)
          .entryPoint('set_treasury')
          .runtimeArgs(Args.fromMap({
              treasury: CLValue.newCLKey(Key.newKey(treasuryAccountHash)),
          }))
          .build();
    }
}
