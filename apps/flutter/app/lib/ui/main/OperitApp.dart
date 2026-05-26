// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../theme/OperitTheme.dart';
import 'screens/OperitMainScreen.dart';

class OperitApp extends StatelessWidget {
  const OperitApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const OperitTheme(child: OperitMainScreen());
  }
}
