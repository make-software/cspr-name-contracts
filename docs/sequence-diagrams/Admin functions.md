# Admin functions

## NameToken admin functions

### Whitelist registrar contract

> As the NameToken contract admin, I should be able to whitelist a Registrar contract, so that it can mint new *cspr name 
> tokens in the registry.
> 
![](puml/whitelist-registrar.png)
[🔗](puml/whitelist-registrar.puml)

_used also to delist a registrar_

### Set transfer filter contract

> As the NameToken contract admin, I should be able to set a transfer filter contract, 
> so that its `can_transfer` method is called before a transfer can be executed.

![](puml/set-transfer-filter.png)
[🔗](puml/set-transfer-filter.puml)
## Registrar admin functions

### Whitelist a controller contract or admin address

> As a Registrar admin, I should be able to whitelist a controller contract or admin account address,
> so that they can mint and renew cspr name tokens, and call admin contract methods.

![](puml/whitelist-controller.png)
[🔗](puml/whitelist-controller.puml)

_used also to delist a controller or admin address._

### Set grace period time span

> As a registrar admin, I should be able to set the duration of the grace period.

![](puml/set-grace-period.png)
[🔗](puml/set-grace-period.puml)

### Expire domains

> As the registrar contract, I should burn expired tokens in batches upon request.

![](puml/expire-domain.png)
[🔗](puml/expire-domain.puml)


### Transfer domains

> As a D3 admin, I should be able to transfer a batch of cspr name tokens.

![](puml/admin-transfer-domain.png)
[🔗](puml/admin-transfer-domain.puml)

### Burn domains

> As a D3 admin, I should be able to burn a batch of cspr name tokens.

![](puml/burn-domain.png)
[🔗](puml/burn-domain.puml)


