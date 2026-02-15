use ethers::contract::abigen;

abigen!(
    NoxStaking,
    r#"[
        function stake(uint256 amount) external returns (bool)
        function unstake(uint256 amount) external returns (bool)
        function claimRewards() external returns (uint256)
        function getStake(address staker) external view returns (uint256)
        function getPendingRewards(address staker) external view returns (uint256)
        function getTier(address staker) external view returns (uint8)
        function setTier(uint8 tier) external returns (bool)
        function registerNode(bytes32 nodeId) external returns (bool)
        function getNodeStake(bytes32 nodeId) external view returns (uint256)
        function isNodeRegistered(bytes32 nodeId) external view returns (bool)
        event Staked(address indexed staker, uint256 amount)
        event Unstaked(address indexed staker, uint256 amount)
        event RewardsClaimed(address indexed staker, uint256 amount)
        event TierChanged(address indexed staker, uint8 tier)
        event NodeRegistered(bytes32 indexed nodeId, address indexed owner)
    ]"#
);

abigen!(
    NoxToken,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transfer(address to, uint256 amount) external returns (bool)
        function totalSupply() external view returns (uint256)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event Approval(address indexed owner, address indexed spender, uint256 value)
    ]"#
);
