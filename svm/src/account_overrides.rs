use {
    scc::HashMap,
    solana_account::AccountSharedData,
    solana_pubkey::Pubkey,
    solana_sdk_ids::sysvar,
};

/// Encapsulates overridden accounts, typically used for transaction
/// simulations. Account overrides are currently not used when loading the
/// durable nonce account or when constructing the instructions sysvar account.
///
/// Uses scc::HashMap for lock-free concurrent access, allowing multiple
/// threads to read/write account overrides simultaneously.
pub struct AccountOverrides {
    accounts: HashMap<Pubkey, AccountSharedData, ahash::RandomState>,
}

impl AccountOverrides {
    /// Insert or remove an account with a given pubkey to/from the list of overrides.
    /// Thread-safe: can be called from multiple threads concurrently.
    pub fn set_account(&self, pubkey: &Pubkey, account: Option<AccountSharedData>) {
        match account {
            Some(account) => {
                let _ = self.accounts.upsert_sync(*pubkey, account);
            }
            None => {
                let _ = self.accounts.remove_sync(pubkey).map(|kv| kv.1);
            }
        };
    }

    /// Sets in the slot history
    ///
    /// Note: no checks are performed on the correctness of the contained data
    pub fn set_slot_history(&self, slot_history: Option<AccountSharedData>) {
        self.set_account(&sysvar::slot_history::id(), slot_history);
    }

    /// Gets the account if it's found in the list of overrides.
    /// Returns an OccupiedEntry which holds a reference to the value.
    pub fn get<'a>(
        &'a self,
        pubkey: &Pubkey,
    ) -> Option<scc::hash_map::OccupiedEntry<'a, Pubkey, AccountSharedData, ahash::RandomState>>
    {
        self.accounts.get_sync(pubkey)
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Returns a reference to the underlying HashMap.
    /// Note: For iteration, use scan() or scan_async() methods on the HashMap.
    pub fn accounts(&self) -> &HashMap<Pubkey, AccountSharedData, ahash::RandomState> {
        &self.accounts
    }

    /// Merge another AccountOverrides into this one.
    /// Thread-safe: can be called while other threads are reading/writing.
    pub fn merge(&self, other: AccountOverrides) {
        other.accounts.iter_sync(|k, v| {
            let _ = self.accounts.upsert_sync(*k, v.clone());
            true // continue iteration
        });
    }
}

impl Default for AccountOverrides {
    fn default() -> AccountOverrides {
        AccountOverrides {
            accounts: HashMap::<_, _, ahash::RandomState>::with_hasher(ahash::RandomState::new()),
        }
    }
}

#[cfg(test)]
mod test {
    use {
        crate::account_overrides::AccountOverrides, solana_account::AccountSharedData,
        solana_pubkey::Pubkey, solana_sdk_ids::sysvar,
    };

    #[test]
    fn test_set_account() {
        let accounts = AccountOverrides::default();
        let data = AccountSharedData::default();
        let key = Pubkey::new_unique();
        accounts.set_account(&key, Some(data.clone()));
        assert_eq!(accounts.get(&key).map(|e| e.get().clone()), Some(data));

        accounts.set_account(&key, None);
        assert!(accounts.get(&key).is_none());
    }

    #[test]
    fn test_slot_history() {
        let accounts = AccountOverrides::default();
        let data = AccountSharedData::default();

        assert!(accounts.get(&sysvar::slot_history::id()).is_none());
        accounts.set_slot_history(Some(data.clone()));

        assert_eq!(
            accounts
                .get(&sysvar::slot_history::id())
                .map(|e| e.get().clone()),
            Some(data)
        );
    }
}
