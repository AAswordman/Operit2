import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/notifications/AppNotificationService.dart';

void main() {
  const now = 1700000000000;

  group('shouldDeliverAppNotification', () {
    test('delivers while the application is in the background', () {
      expect(
        shouldDeliverAppNotification(
          appForeground: false,
          messageTimestamp: now,
          nowMillis: now,
        ),
        isTrue,
      );
    });

    test('suppresses a fresh notification while foregrounded', () {
      expect(
        shouldDeliverAppNotification(
          appForeground: true,
          messageTimestamp: now - 1000,
          nowMillis: now,
        ),
        isFalse,
      );
    });

    test('delivers a replayed notification produced while away', () {
      expect(
        shouldDeliverAppNotification(
          appForeground: true,
          messageTimestamp: now - 60000,
          nowMillis: now,
        ),
        isTrue,
      );
    });

    test('keeps suppressing foreground notifications without a timestamp', () {
      expect(
        shouldDeliverAppNotification(
          appForeground: true,
          messageTimestamp: null,
          nowMillis: now,
        ),
        isFalse,
      );
    });

    test('honors a custom replay threshold', () {
      final delivered = shouldDeliverAppNotification(
        appForeground: true,
        messageTimestamp: now - 2000,
        nowMillis: now,
        awayReplayThreshold: const Duration(seconds: 1),
      );
      expect(delivered, isTrue);
    });
  });
}
