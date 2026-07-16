<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0"
    xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:dmn="https://www.omg.org/spec/DMN/20191111/MODEL/">
  <xsl:output method="html" indent="yes"/>

  <!-- Entry: the wrapper names the DMN; we load it read-only via document(). -->
  <xsl:template match="/render">
    <html lang="en">
      <head>
        <meta charset="UTF-8"/>
        <title>Decision table</title>
        <style>
          table.decision-table { border-collapse: collapse; }
          .decision-table th, .decision-table td { border: 1px solid #c9ced6; padding: .35rem .75rem; text-align: left; }
          .decision-table thead th { background: #eef2f7; }
          .decision-table th:last-child { background: #d9e3ee; }
        </style>
      </head>
      <body>
        <xsl:apply-templates select="document(@dmn)//dmn:decisionTable[1]"/>
      </body>
    </html>
  </xsl:template>

  <xsl:template match="dmn:decisionTable">
    <table class="decision-table">
      <caption>
        <xsl:value-of select="ancestor::dmn:decision/@name"/>
        <xsl:text> (hit policy: </xsl:text>
        <xsl:value-of select="translate(@hitPolicy, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')"/>
        <xsl:text>)</xsl:text>
      </caption>
      <thead>
        <tr>
          <th scope="col"><xsl:value-of select="substring(@hitPolicy, 1, 1)"/></th>
          <xsl:for-each select="dmn:input">
            <th scope="col"><xsl:value-of select="dmn:inputExpression/dmn:text"/></th>
          </xsl:for-each>
          <xsl:for-each select="dmn:output">
            <th scope="col"><xsl:value-of select="@name | ancestor::dmn:decision/dmn:variable/@name"/></th>
          </xsl:for-each>
        </tr>
      </thead>
      <tbody>
        <xsl:for-each select="dmn:rule">
          <tr>
            <td><xsl:value-of select="position()"/></td>
            <xsl:for-each select="dmn:inputEntry | dmn:outputEntry">
              <td><xsl:call-template name="cell"><xsl:with-param name="t" select="dmn:text"/></xsl:call-template></td>
            </xsl:for-each>
          </tr>
        </xsl:for-each>
      </tbody>
    </table>
  </xsl:template>

  <xsl:template name="cell">
    <xsl:param name="t"/>
    <xsl:choose>
      <xsl:when test="$t = '-'">any</xsl:when>
      <xsl:otherwise><xsl:value-of select="translate($t, '&quot;', '')"/></xsl:otherwise>
    </xsl:choose>
  </xsl:template>
</xsl:stylesheet>
