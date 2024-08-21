/* eslint-disable eslint-comments/disable-enable-pair */
/* eslint-disable max-classes-per-file */

import { CLAccountHash, CLByteArray, CLKey, CLKeyBytesParser, CLString, CLStringBytesParser, CLU32,CLU32BytesParser, CLU64, CLU64BytesParser, CLU512, CLU512BytesParser, decodeBase16 } from 'casper-js-sdk';

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

    const bytes = Array.from(buyer);
    bytes.concat(Array.from(paymentId));
    bytes.concat(Array.from(amount));

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
    const tokenExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.tokenExpiration.getMilliseconds())).unwrap()

    const bytes = Array.from(labelBytes);
    bytes.concat(Array.from(ownerBytes));
    bytes.concat(Array.from(tokenExpirationBytes));

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

    const bytes = Array.from(this.payment.toBytes())

    const sizeBytes = new CLU32BytesParser().toBytes(new CLU32(this.names.length)).unwrap()
    bytes.concat(Array.from(sizeBytes))

    this.names.forEach(name => bytes.concat(Array.from(name.toBytes())))
    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
    bytes.concat(Array.from(voucherExpirationBytes))

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

    const sizeBytes = new CLU32BytesParser().toBytes(new CLU32(this.names.length)).unwrap()

    const bytes = Array.from(sizeBytes);
    this.names.forEach(name => bytes.concat(Array.from(name.toBytes())))

    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
    bytes.concat(Array.from(voucherExpirationBytes))

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
    bytes.concat(Array.from(tokenExpirationBytes))

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

    const bytes = Array.from(this.payment.toBytes())

    const sizeBytes = new CLU32BytesParser().toBytes(new CLU32(this.tokens.length)).unwrap()
    bytes.concat(Array.from(sizeBytes))

    this.tokens.forEach(token => bytes.concat(Array.from(token.toBytes())))
    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
    bytes.concat(Array.from(voucherExpirationBytes))

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

    const sizeBytes = new CLU32BytesParser().toBytes(new CLU32(this.tokens.length)).unwrap()

    const bytes = Array.from(sizeBytes)
    this.tokens.forEach(token => bytes.concat(Array.from(token.toBytes())))

    const voucherExpirationBytes = new CLU64BytesParser().toBytes(new CLU64(this.voucherExpiration.getMilliseconds())).unwrap()
    bytes.concat(Array.from(voucherExpirationBytes))

    this.bytes = Uint8Array.from(bytes)

    return this.bytes
  }
}
