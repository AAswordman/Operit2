// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../runtime/RuntimeSettingsPanel.dart';
import '../web_access/WebAccessSettingsPanel.dart';

class AccessLinksSettingsPanel extends StatelessWidget {
  const AccessLinksSettingsPanel({super.key});

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: <Widget>[
        const RuntimeSettingsPanel(embedded: true),
        const SizedBox(height: 2),
        const WebAccessSettingsPanel(embedded: true),
      ],
    );
  }
}
