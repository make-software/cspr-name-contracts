export interface PaymentInfo {
  buyer: string
  payment_id: string
  amount: number
}

interface NameMintInfo {
  label: string
  owner: string
  token_expiration: Date
}

export interface PaymentVoucher {
  payment: PaymentInfo
  names: NameMintInfo[]
  voucher_expiration: Date
}

export interface TokenizationVoucher {
  names: NameMintInfo[]
  voucher_expiration: Date
}

interface TokenRenewalInfo {
  token_id: string
  token_expiration: Date
}

export interface RenewalPaymentVoucher {
  payment: PaymentInfo
  tokens: TokenRenewalInfo[]
  voucher_expiration: Date
}

export interface RenewalVoucher {
  tokens: TokenRenewalInfo[]
  voucher_expiration: Date
}
