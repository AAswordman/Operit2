// ignore_for_file: file_names

import '../../../../../core/chat/OperitChatRuntime.dart';
import '../../../../../util/ChatMarkupRegex.dart';

sealed class MarkdownGroupedItem {
  const MarkdownGroupedItem();
}

class MarkdownSingleItem extends MarkdownGroupedItem {
  const MarkdownSingleItem(this.index);

  final int index;
}

class MarkdownGroupItem extends MarkdownGroupedItem {
  const MarkdownGroupItem({
    required this.startIndex,
    required this.endIndexInclusive,
    required this.stableKey,
  });

  final int startIndex;
  final int endIndexInclusive;
  final String stableKey;
}

List<MarkdownGroupedItem> groupThinkToolsXmlNodes(
  List<ChatMarkdownBlockNode> nodes, {
  bool showThinkingProcess = true,
}) {
  final out = <MarkdownGroupedItem>[];
  var i = 0;
  while (i < nodes.length) {
    final node = nodes[i];
    if (node.nodeType != 'XmlBlock') {
      out.add(MarkdownSingleItem(i));
      i++;
      continue;
    }

    final tag = extractXmlTagName(node.content.toString());
    if (showThinkingProcess && (tag == 'think' || tag == 'thinking')) {
      var j = i + 1;
      var toolCount = 0;
      var xmlToolRelatedCount = 0;
      while (j < nodes.length) {
        final next = nodes[j];
        if (next.nodeType != 'XmlBlock') {
          break;
        }
        final nextTag = extractXmlTagName(next.content.toString());
        if (nextTag == 'meta') {
          j++;
          continue;
        }
        final isThinkAgain = nextTag == 'think' || nextTag == 'thinking';
        final isToolRelated = nextTag == 'tool' || nextTag == 'tool_result';
        if (!isThinkAgain && !isToolRelated) {
          break;
        }
        if (isToolRelated) {
          if (nextTag == 'tool') {
            toolCount++;
          }
          xmlToolRelatedCount++;
        }
        j++;
      }
      if (toolCount >= 2 && xmlToolRelatedCount >= 2) {
        out.add(
          MarkdownGroupItem(
            startIndex: i,
            endIndexInclusive: j - 1,
            stableKey: 'think-tools-$i',
          ),
        );
        i = j;
        continue;
      }
      out.add(MarkdownSingleItem(i));
      i++;
      continue;
    }

    if (tag == 'tool' || tag == 'tool_result') {
      var j = i + 1;
      var toolCount = tag == 'tool' ? 1 : 0;
      var xmlToolRelatedCount = 1;
      while (j < nodes.length) {
        final next = nodes[j];
        if (next.nodeType != 'XmlBlock') {
          break;
        }
        final nextTag = extractXmlTagName(next.content.toString());
        if (nextTag == 'meta') {
          j++;
          continue;
        }
        if (nextTag != 'tool' && nextTag != 'tool_result') {
          break;
        }
        if (nextTag == 'tool') {
          toolCount++;
        }
        xmlToolRelatedCount++;
        j++;
      }
      if (toolCount >= 2 && xmlToolRelatedCount >= 2) {
        out.add(
          MarkdownGroupItem(
            startIndex: i,
            endIndexInclusive: j - 1,
            stableKey: 'tools-only-$i',
          ),
        );
        i = j;
        continue;
      }
    }

    out.add(MarkdownSingleItem(i));
    i++;
  }
  return out;
}

String? extractXmlTagName(String xml) {
  final tag = ChatMarkupRegex.extractOpeningTagName(xml);
  return ChatMarkupRegex.normalizeToolLikeTagName(tag)?.toLowerCase();
}
