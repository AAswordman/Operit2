package app.operit

import app.operit.accessibility.AccessibilityRiskConsent
import org.junit.Assert.*
import org.junit.Test

/** Verifies that the risk acknowledgment requires the exact displayed text. */
class AccessibilityRiskConsentTest {
    /** Accepts only the complete Chinese acknowledgment, with its original punctuation. */
    @Test
    fun chineseAcknowledgmentRequiresEveryCharacter() {
        val expected = AccessibilityRiskConsent.CHINESE_ACKNOWLEDGMENT
        assertTrue(AccessibilityRiskConsent.matches(expected, expected))
        assertFalse(AccessibilityRiskConsent.matches(" $expected", expected))
        assertFalse(AccessibilityRiskConsent.matches("$expected ", expected))
        assertFalse(AccessibilityRiskConsent.matches(expected.replace("，", ","), expected))
        assertFalse(AccessibilityRiskConsent.matches(expected.dropLast(1), expected))
        assertFalse(AccessibilityRiskConsent.matches("", expected))
    }

    /** Rejects case changes, spacing changes and a different language's statement. */
    @Test
    fun englishAcknowledgmentRequiresExactCaseAndSpacing() {
        val expected = AccessibilityRiskConsent.ENGLISH_ACKNOWLEDGMENT
        assertTrue(AccessibilityRiskConsent.matches(expected, expected))
        assertFalse(AccessibilityRiskConsent.matches(expected.lowercase(), expected))
        assertFalse(AccessibilityRiskConsent.matches(expected.replaceFirst(" ", "  "), expected))
        assertFalse(AccessibilityRiskConsent.matches(AccessibilityRiskConsent.CHINESE_ACKNOWLEDGMENT, expected))
    }
}
