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

export interface PaymentVaucher {
  payment: PaymentInfo
  names: NameMintInfo[]
  voucher_expiration: Date
}

export interface TokenizationVaucher {
  names: NameMintInfo[]
  voucher_expiration: Date
}

interface TokenRenewalInfo {
  token_id: string
  token_expiration: Date
}

export interface RenewalPaymentVaucher {
  payment: PaymentInfo
  tokens: TokenRenewalInfo[]
  voucher_expiration: Date
}

export interface RenewalVaucher {
  tokens: TokenRenewalInfo[]
  voucher_expiration: Date
}
