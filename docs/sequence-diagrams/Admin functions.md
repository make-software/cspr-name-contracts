# Admin functions

## Registry admin functions

### Whitelist registrar contract

> As the Registry contract admin, I should be able to whitelist a Registrar contract, so that it can min new *cspr name 
> tokens in the registry.
> 
![](puml/whitelist-registrar.png)
[🔗](puml/whitelist-registrar.puml)

_used also to delist a registrar_

## Registrar admin functions

### Expire a domain

> As the registrar contract, I should burn expired tokens upon request.

![](puml/expire-domain.png)
[🔗](puml/expire-domain.puml)

### Burn a domain

> As a D3 admin, I should be able to burn a cspr name token.

![](puml/burn-domain.png)
[🔗](puml/burn-domain.puml)

### Whitelist a controller contract

> As a Registrar admin, I should be able to whitelist a controller contract, 
> so that it can mint and renew cspr name tokens.

![](puml/whitelist-controller.png)
[🔗](puml/whitelist-controller.puml)

_used also to delist a controller._

