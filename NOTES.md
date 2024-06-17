# TODO

## Name Token
- [ ] - Cleanup resolver on burn and transfer
- [ ] - Approve for all
- [ ] - Transfer
- [ ] - Transfer from operator
- [ ] - Prevent transfer of expired domains

## Registrar
- [x] - Whitelist registrar contract
- [x] - Whitelist a controller contract or admin address
- [x] - Set grace period time span
- [x] - Expire domains
- [x] - Transfer domains
- [x] - Burn domains
- [x] - Buy with fiat currency
- [x] - Renew with fiat currency

## Controller
- [ ] - Buy with CSPR token
- [ ] - Renew with CSPR token
- [ ] - Set a resolver
- [ ] - Set Treasury
- [ ] - Set Verification Key

## Resolver
- [ ] - Set account address record

### Questions
- Should there be multiple registrar admins?
- CEP78 total supply is not decremented when the tokens are burned.
- How is the PublicKey maintained in the controller?