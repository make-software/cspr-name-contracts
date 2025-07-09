export const config = {
  adminPrivateKeyPath: process.env.ADMIN_PRIVATE_KEY_PAIR_PATH,
  buyerPrivateKeyPath: process.env.BUYER_PRIVATE_KEY_PAIR_PATH,
  mintingName: process.env.MINTING_NAME,
  registrarContractPackageHash: process.env.REGISTRAR_CONTRACT_PACKAGE_HASH,
  defaultResolverContractPackageHash: process.env.DEFAULT_RESOLVER_CONTRACT_PACKAGE_HASH,
  reverseResolutionContractPackageHash: process.env.REVERSE_RESOLUTION_CONTRACT_PACKAGE_HASH,
  controllerContractPackageHash: process.env.CONTROLLER_CONTRACT_PACKAGE_HASH,
  nameTokenURL: process.env.NAME_TOKEN_URL,
  networkName: process.env.NETWORK_NAME,
  nodeAddress: process.env.NODE_ADDRESS,
};
