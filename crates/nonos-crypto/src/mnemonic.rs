use bip39::{Language, Mnemonic};
use nonos_types::{Blake3Key, NonosError};
use zeroize::Zeroize;

pub fn generate_mnemonic() -> Result<String, NonosError> {
    let mut entropy = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut entropy);

    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    entropy.zeroize();

    Ok(mnemonic.words().collect::<Vec<_>>().join(" "))
}

pub fn validate_mnemonic(phrase: &str) -> Result<(), NonosError> {
    Mnemonic::parse_in_normalized(Language::English, phrase)
        .map_err(|e| NonosError::InvalidMnemonic(e.to_string()))?;
    Ok(())
}

pub fn mnemonic_to_entropy(phrase: &str) -> Result<Vec<u8>, NonosError> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
        .map_err(|e| NonosError::InvalidMnemonic(e.to_string()))?;
    Ok(mnemonic.to_entropy().to_vec())
}

pub fn derive_blake3_key_from_mnemonic(phrase: &str) -> Result<Blake3Key, NonosError> {
    let entropy = mnemonic_to_entropy(phrase)?;
    Ok(crate::derive_wallet_master_key(&entropy))
}

pub fn mnemonic_to_seed(phrase: &str, passphrase: &str) -> Result<[u8; 64], NonosError> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
        .map_err(|e| NonosError::InvalidMnemonic(e.to_string()))?;
    Ok(mnemonic.to_seed(passphrase))
}

pub struct SecureMnemonic {
    phrase: String,
}

impl SecureMnemonic {
    pub fn new() -> Result<Self, NonosError> {
        let phrase = generate_mnemonic()?;
        Ok(Self { phrase })
    }

    pub fn from_phrase(phrase: String) -> Result<Self, NonosError> {
        validate_mnemonic(&phrase)?;
        Ok(Self { phrase })
    }

    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    pub fn word(&self, index: usize) -> Option<&str> {
        self.phrase.split_whitespace().nth(index)
    }

    pub fn words(&self) -> Vec<&str> {
        self.phrase.split_whitespace().collect()
    }

    pub fn derive_seed(&self, passphrase: &str) -> Result<[u8; 64], NonosError> {
        mnemonic_to_seed(&self.phrase, passphrase)
    }
}

impl Drop for SecureMnemonic {
    fn drop(&mut self) {
        self.phrase.zeroize();
    }
}

pub fn is_valid_bip39_word(word: &str) -> bool {
    Language::English.find_word(word).is_some()
}

pub fn suggest_words(prefix: &str, max_suggestions: usize) -> Vec<&'static str> {
    let wordlist = Language::English.word_list();
    wordlist
        .iter()
        .filter(|word| word.starts_with(prefix))
        .take(max_suggestions)
        .copied()
        .collect()
}

pub fn get_word_at_index(index: usize) -> Option<&'static str> {
    let wordlist = Language::English.word_list();
    wordlist.get(index).copied()
}

pub fn get_word_index(word: &str) -> Option<usize> {
    Language::English.find_word(word).map(|i| i as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonos_types::MNEMONIC_WORD_COUNT;

    #[test]
    fn test_generate_mnemonic() {
        let phrase = generate_mnemonic().unwrap();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), MNEMONIC_WORD_COUNT);
    }

    #[test]
    fn test_validate_mnemonic() {
        let phrase = generate_mnemonic().unwrap();
        assert!(validate_mnemonic(&phrase).is_ok());
        assert!(validate_mnemonic("invalid mnemonic phrase").is_err());
    }

    #[test]
    fn test_entropy_roundtrip() {
        let phrase = generate_mnemonic().unwrap();
        let entropy = mnemonic_to_entropy(&phrase).unwrap();
        assert_eq!(entropy.len(), 32);
    }

    #[test]
    fn test_secure_mnemonic() {
        let secure = SecureMnemonic::new().unwrap();
        assert_eq!(secure.words().len(), MNEMONIC_WORD_COUNT);
    }

    #[test]
    fn test_word_suggestions() {
        let suggestions = suggest_words("ab", 5);
        assert!(!suggestions.is_empty());
        for word in suggestions {
            assert!(word.starts_with("ab"));
        }
    }
}
