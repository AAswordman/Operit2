// ignore_for_file: file_names

import 'onboarding/OnboardingStartupRoute.dart';
import '../main/navigation/StartupRouteStrategy.dart';

class FeatureStartupRoutes {
  const FeatureStartupRoutes._();

  static void registerAll(StartupRouteRegistry registry) {
    registerOnboardingStartupRoute(registry);
  }
}
