use operit_util::FileUtils::FileUtils;

#[test]
fn names_and_each_directory_component_must_be_portable() {
    for name in ["photo.jpg", "中文 图片.png", ".gitignore", "COM0.txt", "COM10.txt", "console", "auxiliary.txt"] {
        assert!(FileUtils::isPortableFileName(name), "{name}");
    }
    for name in ["", ".", "..", "query?.jpg", "a*b", "a:b", "a<b", "a>b", "a|b", "a\"b", "a/b", "a\\b", "a\0b", "a\nb", "trailing.", "trailing ", "CON", "con.txt", "NUL.txt", "aux .txt", "prn", "CONIN$", "conout$.txt", "COM1.txt", "Lpt9", "COM¹.txt", "lpt²", "COM³"] {
        assert!(!FileUtils::isPortableFileName(name), "{name:?}");
    }
    assert!(FileUtils::isPortableFileName(&"a".repeat(255)));
    assert!(!FileUtils::isPortableFileName(&"a".repeat(256)));
    assert!(!FileUtils::isPortableFileName(&"中".repeat(86)));
    assert!(FileUtils::isPortableFilePath("workspaces/项目/file.txt"));
    for path in ["workspaces/CON/file.txt", "workspaces/bad?/file.txt", "workspaces/p/../file", "/workspaces/p/file", "workspaces//file"] {
        assert!(!FileUtils::isPortableFilePath(path), "{path}");
    }
}
