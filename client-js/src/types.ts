/* eslint-disable eslint-comments/disable-enable-pair */
/* eslint-disable max-classes-per-file */

import { CLValue, Key, toBytesU32, toBytesU512, toBytesU64 } from 'casper-js-sdk';

export class PaymentInfo {
  constructor(
    public buyer: string,
    public paymentId: string,
    public amount: number,
  ) {}

  toBytes(): Uint8Array {
    const buyer = CLValue.newCLKey(Key.newKey(this.buyer)).bytes()
    const paymentId = CLValue.newCLString(this.paymentId).bytes()
    const amount = toBytesU512(this.amount)

    const bytes = Array.from(buyer)
      .concat(Array.from(paymentId))
      .concat(Array.from(amount));

    return Uint8Array.from(bytes);
  }
}

export class NameMintInfo {
  constructor(
    public label: string,
    public owner: string,
    public tokenExpiration: Date,
  ) {}

  toBytes(): Uint8Array {
    const labelBytes = CLValue.newCLString(this.label).bytes()
    const ownerBytes = CLValue.newCLKey(Key.newKey(this.owner)).bytes()
    const tokenExpirationBytes = toBytesU64(this.tokenExpiration.getTime()*1000)

    const bytes = Array.from(labelBytes)
      .concat(Array.from(ownerBytes))
      .concat(Array.from(tokenExpirationBytes));

    return Uint8Array.from(bytes);
  }
}

export class PaymentVoucher {
  private bytes: Uint8Array = null

  constructor(
    private readonly payment: PaymentInfo,
    private readonly names: NameMintInfo[],
    private readonly voucherExpiration: Date,
  ) {}

  toBytes(): Uint8Array {
    if (this.bytes) {
      return this.bytes;
    }
    let bytes = Array.from(this.payment.toBytes())

    const sizeBytes = toBytesU32(this.names.length)
    bytes = bytes.concat(Array.from(sizeBytes))

    bytes = this.names.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    // Expiration bytes in microseconds
    const voucherExpirationBytes = toBytesU64(this.voucherExpiration.getTime()*1000)
    bytes = bytes.concat(Array.from(voucherExpirationBytes))

    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}

export class TokenizationVoucher {
  private bytes: Uint8Array = null

  constructor(
    private readonly names: NameMintInfo[],
    private readonly voucherExpiration: Date,
  ) {}
  
  toBytes(): Uint8Array {
    if (this.bytes) {
      return this.bytes;
    }

    const nameInfosLengthBytes = toBytesU32(this.names.length)

    // Add name infos length bytes
    let bytes = Array.from(nameInfosLengthBytes);

    // Add name infos bytes
    bytes = this.names.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    // Add expiration bytes in microseconds
    const voucherExpirationBytes = toBytesU64(this.voucherExpiration.getTime()*1000)
    bytes = bytes.concat(Array.from(voucherExpirationBytes))

    // Memoize and return
    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}

export class TokenRenewalInfo {
  constructor(
    public tokenId: string,
    public tokenExpiration: Date,
  ) {}

  toBytes(): Uint8Array {
    const labelBytes = CLValue.newCLString(this.tokenId).bytes()
    const tokenExpirationBytes = toBytesU64(this.tokenExpiration.getTime()*1000)

    const bytes = Array.from(labelBytes)
      .concat(Array.from(tokenExpirationBytes))

    return Uint8Array.from(bytes);
  }
}

export class RenewalPaymentVoucher {
  private bytes: Uint8Array = null

  constructor(
    private readonly payment: PaymentInfo,
    private readonly tokens: TokenRenewalInfo[],
    private readonly voucherExpiration: Date,
  ) {}

  toBytes(): Uint8Array {
    if (this.bytes) {
      return this.bytes;
    }

    const paymentInfoBytes = Array.from(this.payment.toBytes())

    const tokensLengthBytes = Array.from(toBytesU32(this.tokens.length))

    // Add payment info bytes
    let bytes = paymentInfoBytes
    // Add tokens length bytes
      .concat(tokensLengthBytes)

    // add token infos bytes
    bytes = this.tokens.reduce((b, token) => b.concat(Array.from(token.toBytes())), bytes)

    // add expiration bytes
    const voucherExpirationBytes = toBytesU64(this.voucherExpiration.getTime()*1000)
    bytes = bytes.concat(Array.from(voucherExpirationBytes))

    // Memoize and return
    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}

export class RenewalVoucher {
  private bytes: Uint8Array = null

  constructor(
    private readonly tokens: TokenRenewalInfo[],
    private readonly voucherExpiration: Date,
  ) {}

  toBytes(): Uint8Array {
    if (this.bytes) {
      return this.bytes;
    }

    const tokenInfosLengthBytes = Array.from(toBytesU32(this.tokens.length))

    // Add token infos length bytes
    let bytes = Array.from(tokenInfosLengthBytes)

    // Add token infos bytes
    bytes = this.tokens.reduce((b, token) => b.concat(Array.from(token.toBytes())), bytes)

    // Add expiration bytes
    const voucherExpirationBytes = toBytesU64(this.voucherExpiration.getTime()*1000)
    bytes = bytes.concat(Array.from(voucherExpirationBytes))

    // Memoize and return
    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}
