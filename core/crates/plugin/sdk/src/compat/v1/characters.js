/**
 * Reads legacy role identities through the character provider's published domain API.
 * @param {<K extends import('../../../../../../../plugins/packages/buildin/character_cards/src/api').DomainOperation>(method: K, payload: import('../../../../../../../plugins/packages/buildin/character_cards/src/api').DomainInput<K>) => Promise<import('../../../../../../../plugins/packages/buildin/character_cards/src/api').DomainOutput<K>>} invoke
 */
function __operitCreateV1Characters(invoke) {
    return {
        /** Lists authoritative role identities without interpreting SoftwareSettings binding fields. */
        async listCharacterCards() {
            var cards = await invoke('character.list', {});
            return { totalCount: cards.length, cards: cards.map(
                /** Projects the historical chat identity record from the actual stored role. */
                card => ({ id: card.id, name: card.name, description: card.description, isDefault: card.isDefault, createdAt: card.createdAt, updatedAt: card.updatedAt }),
            ) };
        },
    };
}
