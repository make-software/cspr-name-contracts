/* eslint-disable eslint-comments/disable-enable-pair */
/* eslint-disable max-classes-per-file */

import { CLAccountHash, CLByteArray, CLKey, CLKeyBytesParser, CLString, CLStringBytesParser, CLType,CLTypeTag,CLU32,CLU32BytesParser, CLU64, CLU64BytesParser, CLU512, CLU512BytesParser, CLValue, decodeBase16 } from 'casper-js-sdk';

export class PaymentInfo {
  constructor(
    public buyer: string,
    public paymentId: string,
    public amount: number,
  ) {}

  toBytes(): Uint8Array {
    const buyer = new CLKeyBytesParser().toBytes(new CLKey(new CLByteArray(decodeBase16(this.buyer)))).unwrap()
    const paymentId = new CLStringBytesParser().toBytes(new CLString(this.paymentId)).unwrap()
    const amount = new CLU512BytesParser().toBytes(new CLU512(this.amount)).unwrap()

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
    const labelBytes = new CLStringBytesParser().toBytes(new CLString(this.label)).unwrap()
    const ownerBytes = new CLKeyBytesParser().toBytes(new CLKey(new CLAccountHash(decodeBase16(this.owner)))).unwrap()
    const tokenExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(6401000000000000)).unwrap()

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

    const sizeBytes = new CLU32BytesParser().toBytes(new CLU32(this.names.length)).unwrap()
    bytes = bytes.concat(Array.from(sizeBytes))

    bytes = this.names.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
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

    const nameInfosLengthBytes = new CLU32BytesParser().toBytes(new CLU32(this.names.length)).unwrap()

    // Add name infos length bytes
    let bytes = Array.from(nameInfosLengthBytes);

    // Add name infos bytes
    bytes = this.names.reduce((b, name) => b.concat(Array.from(name.toBytes())), bytes)

    // Add expiration bytes
    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
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
    const labelBytes = new CLStringBytesParser().toBytes(new CLString(this.tokenId)).unwrap()
    const tokenExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.tokenExpiration.getMilliseconds())).unwrap()

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
    const tokensLengthBytes = Array.from(new CLU32BytesParser().toBytes(new CLU32(this.tokens.length)).unwrap())

    // Add payment info bytes
    let bytes = paymentInfoBytes
    // Add tokens length bytes
      .concat(tokensLengthBytes)

    // add token infos bytes
    bytes = this.tokens.reduce((b, token) => b.concat(Array.from(token.toBytes())), bytes)

    // add expiration bytes
    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
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

    const tokenInfosLengthBytes = new CLU32BytesParser().toBytes(new CLU32(this.tokens.length)).unwrap()

    // Add token infos length bytes
    let bytes = Array.from(tokenInfosLengthBytes)

    // Add token infos bytes
    bytes = this.tokens.reduce((b, token) => b.concat(Array.from(token.toBytes())), bytes)

    // Add expiration bytes
    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
    bytes = bytes.concat(Array.from(voucherExpirationBytes))

    // Memoize and return
    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}

export class CLAnyType extends CLType {
  linksTo = "ByteArray";

  tag = CLTypeTag.Any;

  // eslint-disable-next-line class-methods-use-this
  toBytes(): Uint8Array {
    return Uint8Array.from([this.tag]);
  }

  // eslint-disable-next-line class-methods-use-this
  toJSON() {
    return "Any";
  }
}


export class CLAny extends CLValue {
  data: Uint8Array;

  /**
   * Constructs a new `CLAny`.
   *
   * @param v The bytes array.
   */
  constructor(v: Uint8Array) {
    super();
    this.data = v;
  }

  // eslint-disable-next-line class-methods-use-this
  clType(): CLType {
    return new CLAnyType();
  }

  value(): Uint8Array {
    return this.data;
  }
}
