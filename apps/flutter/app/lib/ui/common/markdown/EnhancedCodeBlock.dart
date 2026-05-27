// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'CanvasMonospaceCodeBlockBody.dart';

class EnhancedCodeBlock extends StatefulWidget {
  const EnhancedCodeBlock({super.key, required this.code, this.language = ''});

  final String code;
  final String language;

  @override
  State<EnhancedCodeBlock> createState() => _EnhancedCodeBlockState();
}

class _EnhancedCodeBlockState extends State<EnhancedCodeBlock> {
  bool autoWrapEnabled = true;
  bool showCopiedToast = false;

  @override
  Widget build(BuildContext context) {
    final lines = widget.code.split('\n');
    const codeBlockBackground = Color(0xFF1E1E1E);
    const toolbarBackground = Color(0xFF252526);
    return Semantics(
      label: widget.language.isEmpty
          ? 'Code block'
          : '${widget.language} Code block',
      child: Container(
        width: double.infinity,
        margin: const EdgeInsets.symmetric(vertical: 2),
        decoration: BoxDecoration(
          color: codeBlockBackground,
          borderRadius: BorderRadius.circular(4),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Container(
              width: double.infinity,
              color: toolbarBackground,
              padding: const EdgeInsets.all(4),
              child: Row(
                children: <Widget>[
                  if (widget.language.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(left: 8),
                      child: Text(
                        widget.language,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: const Color(0xFFAAAAAA),
                        ),
                      ),
                    ),
                  const Spacer(),
                  IconButton(
                    onPressed: () {
                      setState(() {
                        autoWrapEnabled = !autoWrapEnabled;
                      });
                    },
                    iconSize: 16,
                    constraints: const BoxConstraints.tightFor(
                      width: 28,
                      height: 28,
                    ),
                    padding: EdgeInsets.zero,
                    color: const Color(0xFFAAAAAA),
                    icon: const Icon(Icons.swap_horiz),
                    tooltip: 'Wrap',
                  ),
                  IconButton(
                    onPressed: () {
                      Clipboard.setData(ClipboardData(text: widget.code));
                      setState(() {
                        showCopiedToast = true;
                      });
                    },
                    iconSize: 16,
                    constraints: const BoxConstraints.tightFor(
                      width: 28,
                      height: 28,
                    ),
                    padding: EdgeInsets.zero,
                    color: showCopiedToast
                        ? Theme.of(context).colorScheme.primary
                        : const Color(0xFFAAAAAA),
                    icon: const Icon(Icons.content_copy),
                    tooltip: 'Copy',
                  ),
                ],
              ),
            ),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 420),
              child: SingleChildScrollView(
                scrollDirection: Axis.vertical,
                child: SingleChildScrollView(
                  scrollDirection: autoWrapEnabled
                      ? Axis.vertical
                      : Axis.horizontal,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: CanvasMonospaceCodeBlockBody(
                      lines: lines,
                      autoWrapEnabled: autoWrapEnabled,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
